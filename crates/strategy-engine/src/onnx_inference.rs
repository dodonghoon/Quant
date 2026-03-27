//! # ONNX Runtime 추론 인터페이스
//!
//! 기술문서 §4.2.3:
//! "Python에서 학습된 모델을 ONNX 포맷으로 내보낸 후,
//!  Rust의 ort 크레이트(ONNX Runtime)로 로드하여 실행."
//!
//! ## 워크플로우
//! ```text
//! Python (research/models/ml_training.py)
//!    │
//!    ├─ PyTorch 모델 학습
//!    │
//!    ├─ torch.onnx.export() → model.onnx
//!    │
//!    └─ 정규화 파라미터 → model.json
//!          │
//!          ▼
//! Rust (이 모듈)
//!    │
//!    ├─ ort::Session::load("model.onnx")
//!    │
//!    ├─ JSON에서 mean/std 로드
//!    │
//!    └─ 실시간 추론 (매 틱마다)
//! ```
//!
//! ## 의존성
//! `ort` 크레이트는 ONNX Runtime C 라이브러리에 의존하므로,
//! 배포 환경에 `libonnxruntime.so`가 설치되어 있어야 합니다.
//! 개발 시에는 `ort` 크레이트가 자동으로 다운로드합니다.

use crate::error::{Result, StrategyError};
use std::path::Path;

/// ONNX 모델 메타데이터 (Python export 시 생성되는 JSON)
#[derive(Debug, Clone)]
pub struct OnnxModelMeta {
    /// 모델 유형 ("lstm" 또는 "transformer")
    pub model_type: String,
    /// 입력 시퀀스 길이
    pub seq_len: usize,
    /// 정규화 평균 (Z-score)
    pub normalize_mean: f64,
    /// 정규화 표준편차 (Z-score)
    pub normalize_std: f64,
}

impl OnnxModelMeta {
    /// JSON 파일에서 로드
    ///
    /// Python의 `PricePredictor.export_onnx()`가 생성하는
    /// `model.json` 파일을 파싱합니다.
    pub fn from_json(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            StrategyError::ConfigError(format!("ONNX meta file read error: {e}"))
        })?;

        // 최소한의 JSON 파싱 (serde_json 사용)
        let v: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            StrategyError::ConfigError(format!("ONNX meta JSON parse error: {e}"))
        })?;

        Ok(Self {
            model_type: v["model_type"]
                .as_str()
                .unwrap_or("lstm")
                .to_string(),
            seq_len: v["seq_len"].as_u64().unwrap_or(60) as usize,
            normalize_mean: v["normalize_mean"].as_f64().unwrap_or(0.0),
            normalize_std: v["normalize_std"].as_f64().unwrap_or(1.0),
        })
    }
}

/// ONNX 추론 엔진
///
/// `ort` 크레이트를 통해 ONNX 모델을 로드하고 실시간 추론을 수행합니다.
/// 이 구조체는 `ort` 크레이트가 Cargo.toml에 추가된 후 완전 구현됩니다.
///
/// ## 현재 상태
/// 인터페이스 정의 완료. `ort` 크레이트 의존성 추가 시 구현 활성화.
pub struct OnnxPredictor {
    meta: OnnxModelMeta,
    /// 입력 버퍼: 최근 seq_len개의 정규화된 값
    input_buffer: Vec<f64>,
    /// 모델 경로
    model_path: String,
    /// 로드 상태
    is_loaded: bool,
}

impl OnnxPredictor {
    /// 새 추론기 생성
    ///
    /// # 인자
    /// - `onnx_path`: ONNX 모델 파일 경로 (e.g., "model.onnx")
    /// - `meta_path`: 메타데이터 JSON 경로 (e.g., "model.json")
    pub fn new(onnx_path: &str, meta_path: &str) -> Result<Self> {
        let meta = OnnxModelMeta::from_json(Path::new(meta_path))?;

        Ok(Self {
            input_buffer: Vec::with_capacity(meta.seq_len),
            model_path: onnx_path.to_string(),
            is_loaded: false,
            meta,
        })
    }

    /// 모델 로드 (ort Session 초기화)
    ///
    /// NOTE: ort 크레이트 추가 후 실제 Session 로드로 교체 예정.
    /// ```rust,ignore
    /// use ort::{Environment, Session, SessionBuilder, Value};
    /// let env = Environment::builder().build()?;
    /// let session = SessionBuilder::new(&env)?.with_model(&self.model_path)?;
    /// ```
    pub fn load(&mut self) -> Result<()> {
        if !Path::new(&self.model_path).exists() {
            return Err(StrategyError::ConfigError(format!(
                "ONNX model not found: {}",
                self.model_path
            )));
        }
        self.is_loaded = true;
        tracing::info!(
            model = %self.model_path,
            seq_len = self.meta.seq_len,
            "ONNX model loaded (interface ready)"
        );
        Ok(())
    }

    /// 새 가격 관측 입력 + 예측
    ///
    /// 1. 정규화: z = (price - mean) / std
    /// 2. 버퍼에 추가 (슬라이딩 윈도우)
    /// 3. seq_len 도달 시 추론 실행
    pub fn update(&mut self, price: f64) -> Result<Option<OnnxPrediction>> {
        // Z-score 정규화 (Python ml_training.py와 동일)
        let normalized = (price - self.meta.normalize_mean) / self.meta.normalize_std;

        // 버퍼에 추가
        self.input_buffer.push(normalized);

        // 윈도우 초과 시 앞쪽 제거
        if self.input_buffer.len() > self.meta.seq_len {
            self.input_buffer.remove(0);
        }

        // 시퀀스 미완성
        if self.input_buffer.len() < self.meta.seq_len {
            return Ok(None);
        }

        // ── 추론 실행 ──
        let prediction = self.run_inference()?;

        // 역정규화
        let predicted_price = prediction * self.meta.normalize_std + self.meta.normalize_mean;

        Ok(Some(OnnxPrediction {
            predicted_price,
            normalized_prediction: prediction,
            current_price: price,
            direction: if predicted_price > price {
                PriceDirection::Up
            } else if predicted_price < price {
                PriceDirection::Down
            } else {
                PriceDirection::Flat
            },
        }))
    }

    /// 실제 ONNX 추론 (ort 크레이트 추가 시 구현)
    ///
    /// 현재는 간단한 선형 외삽으로 대체 (플레이스홀더).
    fn run_inference(&self) -> Result<f64> {
        // TODO: ort::Session::run() 호출로 교체
        //
        // ```rust,ignore
        // let input_tensor = ndarray::Array3::from_shape_vec(
        //     (1, self.meta.seq_len, 1),
        //     self.input_buffer.clone(),
        // )?;
        // let outputs = self.session.run(vec![Value::from_array(input_tensor)?])?;
        // let pred = outputs[0].try_extract::<f64>()?;
        // ```

        // 플레이스홀더: 마지막 값 + 최근 모멘텀
        let n = self.input_buffer.len();
        if n < 2 {
            return Ok(*self.input_buffer.last().unwrap_or(&0.0));
        }
        let momentum = self.input_buffer[n - 1] - self.input_buffer[n - 2];
        Ok(self.input_buffer[n - 1] + momentum * 0.5)
    }

    /// 모델 정보
    pub fn info(&self) -> &OnnxModelMeta {
        &self.meta
    }
}

/// ONNX 추론 결과
#[derive(Debug, Clone)]
pub struct OnnxPrediction {
    /// 예측 가격 (역정규화)
    pub predicted_price: f64,
    /// 정규화된 예측값
    pub normalized_prediction: f64,
    /// 현재 가격
    pub current_price: f64,
    /// 예측 방향
    pub direction: PriceDirection,
}

/// 가격 방향 예측
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PriceDirection {
    Up,
    Down,
    Flat,
}
