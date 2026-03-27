//! # 엔진 브릿지
//!
//! Rust 트레이딩 엔진 ↔ 웹 API 연결 계층.
//! 데모 모드에서는 목 데이터를 제공합니다.

pub mod engine_bridge;
pub mod exec_bridge;
pub mod feed_bridge;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// 전체 엔진 브릿지 (AppState에서 공유)
pub struct EngineBridge {
    /// 시스템 상태
    pub system: Arc<RwLock<SystemState>>,
    /// 포지션 정보
    pub positions: Arc<RwLock<HashMap<String, PositionInfo>>>,
    /// 활성 주문
    pub orders: Arc<RwLock<Vec<OrderInfo>>>,
    /// 체결 내역
    pub fills: Arc<RwLock<Vec<FillInfo>>>,
    /// 최근 시그널
    pub signals: Arc<RwLock<Vec<SignalInfo>>>,
    /// 모델 상태
    pub models: Arc<RwLock<HashMap<String, ModelState>>>,
    /// 페어 목록
    pub pairs: Arc<RwLock<Vec<PairInfo>>>,
    /// Kill Switch 상태
    pub kill_switch: Arc<RwLock<KillSwitchState>>,
    /// 전략 설정
    pub signal_config: Arc<RwLock<SignalConfigDto>>,
    /// 리스크 설정
    pub risk_config: Arc<RwLock<RiskConfigDto>>,
    /// Kelly 설정
    pub kelly_config: Arc<RwLock<KellyConfigDto>>,
    /// Kalman 설정
    pub kalman_config: Arc<RwLock<KalmanConfigDto>>,
    /// GARCH 설정
    pub garch_config: Arc<RwLock<GarchConfigDto>>,
    /// Almgren-Chriss 설정
    pub ac_config: Arc<RwLock<AlmgrenChrissConfigDto>>,
    /// 실시간 이벤트 브로드캐스트
    pub event_tx: broadcast::Sender<DashboardEvent>,
}

// ── DTO 타입들 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub feed: String,
    pub strategy: String,
    pub execution: String,
    pub kill_switch: bool,
    pub uptime_secs: u64,
    pub latency_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    pub symbol: String,
    pub quantity: f64,
    pub avg_entry: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    pub internal_id: u64,
    pub exchange_id: Option<String>,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub quantity: f64,
    pub price: f64,
    pub status: String,
    pub filled_qty: f64,
    pub avg_fill_price: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillInfo {
    pub internal_id: u64,
    pub exchange_id: String,
    pub symbol: String,
    pub side: String,
    pub filled_qty: f64,
    pub fill_price: f64,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalInfo {
    pub pair: String,
    pub direction: String,
    pub composite_z: f64,
    pub confidence: f64,
    pub raw_position_frac: f64,
    pub ou_z: f64,
    pub kalman_innovation: f64,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelState {
    pub kalman: Option<KalmanState>,
    pub ou: Option<OuState>,
    pub garch: Option<GarchState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KalmanState {
    pub estimated_price: f64,
    pub gain: f64,
    pub innovation: f64,
    pub estimation_error: f64,
    pub tick_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuState {
    pub z_score: f64,
    pub spread: f64,
    pub is_mean_reverting: bool,
    pub kappa: f64,
    pub mu: f64,
    pub sigma: f64,
    pub half_life: f64,
    pub r_squared: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarchState {
    pub variance: f64,
    pub volatility: f64,
    pub long_run_volatility: f64,
    pub persistence: f64,
    pub sample_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairInfo {
    pub id: String,
    pub leg_a: String,
    pub leg_b: String,
    pub hedge_ratio: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitchState {
    pub active: bool,
    pub reason: Option<String>,
    pub activated_at: Option<String>,
}

// ── Config DTOs ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfigDto {
    pub entry_threshold: f64,
    pub strong_entry_threshold: f64,
    pub exit_threshold: f64,
    pub ou_weight: f64,
    pub kalman_weight: f64,
    pub min_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfigDto {
    pub max_daily_loss: f64,
    pub max_position_per_symbol: f64,
    pub max_total_exposure: f64,
    pub max_order_size: f64,
    pub max_orders_per_second: u32,
    pub max_consecutive_failures: u32,
    pub total_capital: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyConfigDto {
    pub kelly_fraction: f64,
    pub max_position_fraction: f64,
    pub min_position_fraction: f64,
    pub risk_free_rate: f64,
    pub min_win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KalmanConfigDto {
    pub process_noise: f64,
    pub measurement_noise: f64,
    pub divergence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarchConfigDto {
    pub omega: f64,
    pub alpha: f64,
    pub beta: f64,
    pub initial_variance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmgrenChrissConfigDto {
    pub permanent_impact: f64,
    pub temporary_impact: f64,
    pub daily_volatility: f64,
    pub risk_aversion: f64,
}

/// 대시보드 실시간 이벤트 (WebSocket으로 팬아웃)
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "channel", content = "data")]
pub enum DashboardEvent {
    Signal(SignalInfo),
    OrderUpdate(OrderInfo),
    Fill(FillInfo),
    RiskUpdate { daily_pnl: f64, exposure: f64, kill_switch: bool },
    SystemMetrics(SystemState),
}

impl EngineBridge {
    /// 데모 모드 — 목 데이터로 초기화
    pub fn new_demo() -> Self {
        let (event_tx, _) = broadcast::channel(1024);

        let mut positions = HashMap::new();
        positions.insert("BTC".to_string(), PositionInfo {
            symbol: "BTC".to_string(), quantity: 0.15, avg_entry: 67100.0,
            unrealized_pnl: 20.18, realized_pnl: 312.0,
        });
        positions.insert("ETH".to_string(), PositionInfo {
            symbol: "ETH".to_string(), quantity: -2.3, avg_entry: 3520.0,
            unrealized_pnl: -4.6, realized_pnl: 28.5,
        });

        let orders = vec![
            OrderInfo {
                internal_id: 1042, exchange_id: Some("binance_1042".to_string()),
                symbol: "BTC".to_string(), side: "Buy".to_string(),
                order_type: "Limit".to_string(), quantity: 0.15, price: 67200.0,
                status: "Sent".to_string(), filled_qty: 0.0, avg_fill_price: 0.0,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        ];

        let signals = vec![
            SignalInfo {
                pair: "BTC-ETH".to_string(), direction: "StrongBuy".to_string(),
                composite_z: -2.7, confidence: 0.89, raw_position_frac: 0.62,
                ou_z: -2.8, kalman_innovation: -1.2,
                ts: chrono::Utc::now().to_rfc3339(),
            },
        ];

        let mut models = HashMap::new();
        models.insert("BTC".to_string(), ModelState {
            kalman: Some(KalmanState {
                estimated_price: 67234.5, gain: 0.032, innovation: -1.2,
                estimation_error: 0.0012, tick_count: 45230,
            }),
            ou: None,
            garch: Some(GarchState {
                variance: 0.00042, volatility: 0.0205, long_run_volatility: 0.018,
                persistence: 0.96, sample_count: 12000,
            }),
        });
        models.insert("BTC-ETH".to_string(), ModelState {
            kalman: None,
            ou: Some(OuState {
                z_score: -1.82, spread: -45.2, is_mean_reverting: true,
                kappa: 0.045, mu: -0.12, sigma: 0.034,
                half_life: 15.4, r_squared: 0.87,
            }),
            garch: None,
        });

        let pairs = vec![
            PairInfo { id: "pair_1".to_string(), leg_a: "BTC".to_string(), leg_b: "ETH".to_string(), hedge_ratio: 0.052, status: "active".to_string() },
            PairInfo { id: "pair_2".to_string(), leg_a: "SOL".to_string(), leg_b: "AVAX".to_string(), hedge_ratio: 0.83, status: "active".to_string() },
        ];

        Self {
            system: Arc::new(RwLock::new(SystemState {
                feed: "connected".to_string(), strategy: "running".to_string(),
                execution: "running".to_string(), kill_switch: false,
                uptime_secs: 0, latency_us: 42,
            })),
            positions: Arc::new(RwLock::new(positions)),
            orders: Arc::new(RwLock::new(orders)),
            fills: Arc::new(RwLock::new(Vec::new())),
            signals: Arc::new(RwLock::new(signals)),
            models: Arc::new(RwLock::new(models)),
            pairs: Arc::new(RwLock::new(pairs)),
            kill_switch: Arc::new(RwLock::new(KillSwitchState { active: false, reason: None, activated_at: None })),
            signal_config: Arc::new(RwLock::new(SignalConfigDto {
                entry_threshold: 1.5, strong_entry_threshold: 2.5, exit_threshold: 0.5,
                ou_weight: 0.7, kalman_weight: 0.3, min_confidence: 0.3,
            })),
            risk_config: Arc::new(RwLock::new(RiskConfigDto {
                max_daily_loss: 1000.0, max_position_per_symbol: 100.0,
                max_total_exposure: 2.0, max_order_size: 10.0,
                max_orders_per_second: 50, max_consecutive_failures: 5,
                total_capital: 100000.0,
            })),
            kelly_config: Arc::new(RwLock::new(KellyConfigDto {
                kelly_fraction: 0.25, max_position_fraction: 0.10,
                min_position_fraction: 0.001, risk_free_rate: 0.05, min_win_rate: 0.50,
            })),
            kalman_config: Arc::new(RwLock::new(KalmanConfigDto {
                process_noise: 1e-5, measurement_noise: 1e-3, divergence_threshold: 50.0,
            })),
            garch_config: Arc::new(RwLock::new(GarchConfigDto {
                omega: 0.0004 * (1.0 - 0.06 - 0.90), alpha: 0.06, beta: 0.90,
                initial_variance: 0.0004,
            })),
            ac_config: Arc::new(RwLock::new(AlmgrenChrissConfigDto {
                permanent_impact: 2.5e-7, temporary_impact: 2.5e-6,
                daily_volatility: 0.02, risk_aversion: 1e-6,
            })),
            event_tx,
        }
    }
}
