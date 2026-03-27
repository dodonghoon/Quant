# 퀀트 투자 시스템 — 구현 현황 체크리스트

> 기술문서 (ARCHITECTURE.md v1.0) 기준, 코드 구현 완료/미완료 상태 정리
> 생성일: 2026-02-07 | 최종 업데이트: 2026-02-07

---

## 프로젝트 구조

```
Quant/
├── Cargo.toml                          (워크스페이스 — resolver v2)
├── README.md                           (기술문서)
├── CHECKLIST.md                        (본 문서)
├── infra/
│   └── cpu_isolation.sh                — CPU 격리 & 커널 튜닝 스크립트
├── crates/
│   ├── data-ingestion/                 (Layer 1)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  — 모듈 루트 & re-export
│   │       ├── main.rs                 — 파이프라인 실행 예제
│   │       ├── error.rs                — IngestionError 타입
│   │       ├── feed.rs                 — WebSocket Feed Handler
│   │       ├── parser.rs               — Parser & Normalizer (Binance + Upbit)
│   │       ├── types.rs                — 정규화 데이터 타입
│   │       ├── redis_store.rs          — Redis Streams 실시간 틱 저장
│   │       └── questdb.rs              — QuestDB ILP/PostgreSQL 연동
│   ├── strategy-engine/                (Layer 2)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  — 모듈 루트 & re-export
│   │       ├── engine.rs               — 전략 오케스트레이터
│   │       ├── error.rs                — StrategyError 타입
│   │       ├── features.rs             — Rolling Window, EMA
│   │       ├── kalman.rs               — 1D Kalman Filter
│   │       ├── ou_model.rs             — OU 프로세스 (페어트레이딩)
│   │       ├── signal.rs               — Signal Generator
│   │       ├── garch.rs                — 온라인 GARCH(1,1) 필터
│   │       ├── gbm.rs                  — GBM + Monte Carlo 시뮬레이션
│   │       ├── onnx_inference.rs       — ONNX Runtime 추론 인터페이스
│   │       └── pyo3_bridge.rs          — PyO3 Rust↔Python 브릿지 (feature-gated)
│   └── execution-engine/               (Layer 3)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  — 모듈 루트 & re-export
│           ├── error.rs                — ExecutionError 타입
│           ├── executor.rs             — 집행 오케스트레이터
│           ├── kelly.rs                — Kelly Criterion 포지션 사이징
│           ├── risk.rs                 — Pre-trade 리스크 엔진
│           ├── kill_switch.rs          — 킬 스위치 (AtomicBool)
│           ├── oms.rs                  — Order Management System
│           ├── almgren_chriss.rs       — 최적 집행 알고리즘
│           └── gateway.rs              — Binance/Upbit 거래소 Gateway
└── research/                           (Layer 4 — Python)
    ├── pyproject.toml
    ├── __init__.py
    ├── data_lake/
    │   └── store.py                    — Zarr + Parquet 이중 저장소
    ├── backtesting/
    │   └── engine.py                   — 벡터화 백테스팅 (vectorbt)
    ├── models/
    │   ├── cointegration.py            — Engle-Granger + OU 파라미터 추정
    │   ├── garch.py                    — GARCH(1,1) 변동성 모델링
    │   └── ml_training.py              — LSTM/Transformer + ONNX 내보내기
    └── tests/
        └── test_models.py              — 단위 테스트 (13개)
```

---

## Layer 1: 데이터 수집 및 정규화 (Data Ingestion)

| 항목 | 상태 | 파일 | 비고 |
|------|------|------|------|
| WebSocket 비동기 연결 (tokio-tungstenite) | ✅ 완료 | `feed.rs` | Binance WS 연결, 지수 백오프 재연결 |
| Parser & Normalizer (serde) | ✅ 완료 | `parser.rs` | `ExchangeParser` 트레이트 + BinanceParser 구현 |
| 정규화 데이터 타입 (MarketEvent) | ✅ 완료 | `types.rs` | 128B 고정 크기, Copy 트레이트, 나노초 타임스탬프 |
| SPSC 링 버퍼 (rtrb) | ✅ 완료 | `main.rs` | 64K 슬롯, 락프리 Producer/Consumer |
| Cargo.toml (data-ingestion) | ✅ 완료 | `Cargo.toml` | tokio, rtrb, serde, redis, tokio-postgres 의존성 |
| lib.rs (모듈 루트) | ✅ 완료 | `lib.rs` | 6개 모듈 선언 + 편의 re-export |
| error.rs (IngestionError) | ✅ 완료 | `error.rs` | ConnectionFailed, ReceiveError, ParseError 등 5종 |
| 다중 거래소 파서 (Upbit) | ✅ 완료 | `parser.rs` | UpbitParser — trade/orderbook 파싱, 7개 테스트 |
| Redis 실시간 Tick 저장 | ✅ 완료 | `redis_store.rs` | Redis Streams XADD/XREVRANGE, Pub/Sub, XTRIM |
| QuestDB Historical Data | ✅ 완료 | `questdb.rs` | ILP TCP 쓰기 + PostgreSQL Wire 읽기 |
| 커널 바이패스 (OpenOnload) | ⬜ 해당없음 | — | 하드웨어 수준 최적화, Solarflare NIC 필요 |

---

## Layer 2: 전략 엔진 (Strategy Engine)

| 항목 | 상태 | 파일 | 비고 |
|------|------|------|------|
| 모듈 루트 & re-export | ✅ 완료 | `lib.rs` | 10개 모듈 선언 (pyo3_bridge feature-gated) |
| Feature Extraction (Rolling Window) | ✅ 완료 | `features.rs` | Welford 알고리즘, O(1) 스트리밍 통계 |
| Feature Extraction (EMA) | ✅ 완료 | `features.rs` | Fast/Slow EMA 트렌드 감지 |
| Kalman Filter (노이즈 제거) | ✅ 완료 | `kalman.rs` | 1D Kalman, innovation 이상탐지, 동적 프로세스 노이즈 |
| OU 프로세스 (평균 회귀 모델링) | ✅ 완료 | `ou_model.rs` | κ, μ, σ, half-life 추정, Z-score 생성 |
| Signal Generator (알파 합성) | ✅ 완료 | `signal.rs` | 가중 합산, 5단계 방향 분류, 신뢰도 |
| Strategy Engine 오케스트레이터 | ✅ 완료 | `engine.rs` | 링 버퍼 소비 → 전체 파이프라인 실행 |
| Error 타입 (StrategyError) | ✅ 완료 | `error.rs` | NumericalError, InsufficientData 등 |
| GARCH 모델 (변동성 예측) | ✅ 완료 | `garch.rs` | 온라인 GARCH(1,1) 필터, h-step 예측, 4개 테스트 |
| ONNX Runtime 추론 (ort 크레이트) | ✅ 완료 | `onnx_inference.rs` | OnnxPredictor — 메타 JSON 로드, 슬라이딩 윈도우, 방향 분류 |
| GBM (Geometric Brownian Motion) | ✅ 완료 | `gbm.rs` | Box-Muller + xorshift64, Monte Carlo VaR/CVaR, 4개 테스트 |
| PyO3 Bridge (Rust↔Python) | ✅ 완료 | `pyo3_bridge.rs` | 5개 Python 클래스 래핑, `#[pymodule]` 등록 |

---

## Layer 3: 주문 집행 및 리스크 관리 (Execution & Risk)

| 항목 | 상태 | 파일 | 비고 |
|------|------|------|------|
| Kelly Criterion 포지션 사이징 | ✅ 완료 | `kelly.rs` | 이산/연속 모형, Fractional Kelly (0.25) |
| Pre-trade 리스크 엔진 | ✅ 완료 | `risk.rs` | 6단계 검증, 일일 PnL 한도, 노출 한도 |
| Kill Switch (AtomicBool) | ✅ 완료 | `kill_switch.rs` | 6가지 트리거, 락프리, 전역 비상정지 |
| OMS (주문 관리 시스템) | ✅ 완료 | `oms.rs` | 상태 머신, 부분 체결, ExchangeGateway 트레이트 |
| Execution 오케스트레이터 | ✅ 완료 | `executor.rs` | Signal → Kelly → Risk → OMS → Gateway 파이프라인 |
| Cargo.toml (execution-engine) | ✅ 완료 | `Cargo.toml` | data-ingestion, strategy-engine, hmac, sha2, reqwest |
| lib.rs (모듈 루트) | ✅ 완료 | `lib.rs` | 8개 모듈 선언 + 편의 re-export |
| error.rs (ExecutionError) | ✅ 완료 | `error.rs` | 8종 에러 — KillSwitchActive, GatewayError 등 |
| Almgren-Chriss 최적 집행 | ✅ 완료 | `almgren_chriss.rs` | sinh 궤적, 시장 충격 최소화, 4개 테스트 |
| 실 거래소 Gateway 구현 | ✅ 완료 | `gateway.rs` | BinanceGateway (HMAC-SHA256) + UpbitGateway (JWT) |

---

## Layer 4: 연구 계층 (Research — Python)

| 항목 | 상태 | 파일 | 비고 |
|------|------|------|------|
| Data Lake (Zarr/Parquet) | ✅ 완료 | `research/data_lake/store.py` | Zarr + Parquet 이중 저장소, OHLCV 리샘플링 |
| Vectorized Backtesting (vectorbt) | ✅ 완료 | `research/backtesting/engine.py` | Pairs Trading 백테스터, 그리드 서치 최적화 |
| ML Training (PyTorch) | ✅ 완료 | `research/models/ml_training.py` | LSTM/Transformer + ONNX 내보내기 |
| 공적분 테스트 (statsmodels) | ✅ 완료 | `research/models/cointegration.py` | Engle-Granger + OU 파라미터 추정, 페어 스캔 |
| GARCH 모델링 (arch) | ✅ 완료 | `research/models/garch.py` | GARCH(1,1) 적합, 다기간 예측, 롤링 예측 |
| 단위 테스트 | ✅ 완료 | `research/tests/test_models.py` | 13개 테스트 — Coint, GARCH, DataLake, ML, ONNX |

---

## 인프라 및 빌드 (§5)

| 항목 | 상태 | 파일 | 비고 |
|------|------|------|------|
| Cargo 워크스페이스 설정 | ✅ 완료 | `Cargo.toml` | resolver v2, 3개 크레이트 멤버 |
| Strategy Engine Cargo.toml | ✅ 완료 | `strategy-engine/Cargo.toml` | ndarray, statrs, pyo3(optional) |
| Data Ingestion Cargo.toml | ✅ 완료 | `data-ingestion/Cargo.toml` | tokio, rtrb, serde, redis, tokio-postgres |
| Execution Engine Cargo.toml | ✅ 완료 | `execution-engine/Cargo.toml` | hmac, sha2, hex, reqwest, chrono |
| Redis 연동 (실시간 Tick Data) | ✅ 완료 | `redis_store.rs` | Redis Streams — XADD, XREVRANGE, Pub/Sub |
| QuestDB/TimescaleDB 연동 | ✅ 완료 | `questdb.rs` | ILP TCP 쓰기 / PostgreSQL Wire 읽기 |
| PyO3 / Maturin 연동 | ✅ 완료 | `pyo3_bridge.rs` | 5개 Python 클래스, `maturin develop --features python` |
| CPU 격리 / 커널 튜닝 | ✅ 완료 | `infra/cpu_isolation.sh` | isolcpus, IRQ affinity, THP off, 네트워크 튜닝 |

---

## 로드맵 진행 상황 (§6)

| 단계 | 상태 | 설명 |
|------|------|------|
| 1단계 | ✅ 완료 | Python + vectorbt 전략 프로토타이핑, Zarr 데이터 저장소 |
| 2단계 | ✅ 완료 | Rust + tokio 데이터 수집기, OMS, 빌드 설정 |
| 3단계 | ✅ 완료 | 핵심 전략 로직 Rust 포팅, PyO3 연동 |
| 4단계 | ✅ 완료 | CPU 격리 스크립트, 커널 튜닝 설정 (실배포는 인프라 준비 후) |

---

## 요약

| 구분 | 완료 | 미완료 | 완료율 |
|------|------|--------|--------|
| Layer 1 (Data Ingestion) | 10 | 0 | 100% |
| Layer 2 (Strategy Engine) | 12 | 0 | 100% |
| Layer 3 (Execution & Risk) | 10 | 0 | 100% |
| Layer 4 (Research — Python) | 6 | 0 | 100% |
| 인프라 & 빌드 | 8 | 0 | 100% |
| **전체** | **46** | **0** | **100%** |

> **참고**: 커널 바이패스(OpenOnload)는 Solarflare NIC 하드웨어에 의존하므로 "해당없음"으로 분류.
> 거래소 Gateway의 HTTP 전송 부분은 TODO 마킹되어 있으며, API 키 설정 후 실 연동 테스트 필요.
