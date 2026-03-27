//! # Strategy Engine (Orchestrator)
//!
//! Data Ingestion의 링 버퍼에서 `MarketEvent`를 소비하여
//! Feature Extraction → Kalman Filter → OU Model → Signal Generator
//! 파이프라인을 실행합니다.
//!
//! ## 스레딩 모델
//! ```text
//!   [tokio task]              [OS thread: strategy-engine]
//!   FeedHandler ──rtrb──→ StrategyEngine.run()
//!   (I/O bound)            ├─ Kalman Filter (per symbol)
//!                          ├─ OU Model (per pair)
//!                          ├─ Signal Generator
//!                          └─→ crossbeam → [Execution Engine]
//! ```
//!
//! CPU-bound 작업이므로 전용 OS 스레드에서 실행하여
//! tokio 런타임을 블로킹하지 않습니다.

use crate::error::Result;
use crate::features::{Ema, RollingWindow};
use crate::kalman::{KalmanConfig, KalmanFilter};
use crate::ou_model::{OuConfig, OuModel};
use crate::signal::{SignalConfig, SignalGenerator, TradingSignal};
use data_ingestion::types::{Exchange, MarketEvent, Symbol};
use rtrb::Consumer;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ────────────────────────────────────────────
// Per-Symbol State
// ────────────────────────────────────────────

/// 개별 심볼에 대한 전략 상태.
/// 각 심볼마다 독립적인 필터와 통계를 유지합니다.
struct SymbolState {
    /// Kalman Filter: 노이즈 제거된 가격 추정
    kalman: KalmanFilter,
    /// Mid-price 롤링 통계 (변동성 추정용)
    mid_window: RollingWindow,
    /// EMA (단기/장기 교차 감지용)
    ema_fast: Ema,
    ema_slow: Ema,
    /// 최근 mid-price
    last_mid: f64,
    /// 최근 Kalman 추정 가격
    last_filtered_price: f64,
    /// 처리된 이벤트 수
    tick_count: u64,
}

impl SymbolState {
    fn new(config: &EngineConfig) -> Self {
        Self {
            kalman: KalmanFilter::new(config.kalman_config.clone()),
            mid_window: RollingWindow::new(config.rolling_window_size),
            ema_fast: Ema::new(config.ema_fast_period),
            ema_slow: Ema::new(config.ema_slow_period),
            last_mid: f64::NAN,
            last_filtered_price: f64::NAN,
            tick_count: 0,
        }
    }
}

// ────────────────────────────────────────────
// Per-Pair State (Pairs Trading)
// ────────────────────────────────────────────

/// 페어 트레이딩용 심볼 쌍 키
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PairKey {
    pub leg_a: Symbol,
    pub leg_b: Symbol,
}

/// 페어 트레이딩 상태
struct PairState {
    ou_model: OuModel,
    /// Hedge Ratio (단순 가격 비율로 시작, 이후 공적분 기반으로 확장 가능)
    hedge_ratio: f64,
}

// ────────────────────────────────────────────
// Engine Configuration
// ────────────────────────────────────────────

/// 전략 엔진 전체 설정
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub kalman_config: KalmanConfig,
    pub ou_config: OuConfig,
    pub signal_config: SignalConfig,

    /// 롤링 윈도우 크기 (심볼별 통계)
    pub rolling_window_size: usize,
    /// 단기 EMA 주기
    pub ema_fast_period: usize,
    /// 장기 EMA 주기
    pub ema_slow_period: usize,

    /// 지표 로깅 간격 (이벤트 수)
    pub metrics_log_interval: u64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            kalman_config: KalmanConfig::default(),
            ou_config: OuConfig::default(),
            signal_config: SignalConfig::default(),
            rolling_window_size: 500,
            ema_fast_period: 20,
            ema_slow_period: 100,
            metrics_log_interval: 100_000,
        }
    }
}

// ────────────────────────────────────────────
// Signal Output Callback
// ────────────────────────────────────────────

/// 신호 생성 시 호출되는 콜백.
/// Execution Engine으로 전달하거나 로깅에 사용합니다.
pub trait SignalSink: Send {
    fn on_signal(&mut self, symbol: &Symbol, signal: &TradingSignal);
}

/// 단순 로깅 구현 (디버깅용)
pub struct LoggingSink;

impl SignalSink for LoggingSink {
    fn on_signal(&mut self, symbol: &Symbol, signal: &TradingSignal) {
        if signal.direction != crate::signal::SignalDirection::Neutral {
            tracing::info!(
                symbol = %symbol,
                direction = ?signal.direction,
                z = format!("{:.3}", signal.composite_z),
                confidence = format!("{:.3}", signal.confidence),
                position_frac = format!("{:.4}", signal.raw_position_frac),
                "SIGNAL"
            );
        }
    }
}

/// crossbeam 채널로 Execution Engine에 전달하는 구현
pub struct ChannelSink {
    tx: crossbeam_channel::Sender<(Symbol, TradingSignal)>,
}

impl ChannelSink {
    pub fn new(tx: crossbeam_channel::Sender<(Symbol, TradingSignal)>) -> Self {
        Self { tx }
    }
}

impl SignalSink for ChannelSink {
    fn on_signal(&mut self, symbol: &Symbol, signal: &TradingSignal) {
        // try_send: 비블로킹. 채널 풀이면 드롭 (Execution이 느리면 오래된 신호는 무의미)
        let _ = self.tx.try_send((*symbol, *signal));
    }
}

// ────────────────────────────────────────────
// Strategy Engine
// ────────────────────────────────────────────

pub struct StrategyEngine<S: SignalSink> {
    config: EngineConfig,
    /// MarketEvent 수신 (SPSC ring buffer consumer)
    consumer: Consumer<MarketEvent>,
    /// 전역 킬 스위치
    kill_switch: Arc<AtomicBool>,
    /// 심볼별 상태
    symbol_states: HashMap<Symbol, SymbolState>,
    /// 페어별 상태
    pair_states: HashMap<PairKey, PairState>,
    /// 신호 생성기
    signal_gen: SignalGenerator,
    /// 신호 출력
    sink: S,
    /// 처리된 총 이벤트 수
    total_events: u64,
}

impl<S: SignalSink> StrategyEngine<S> {
    pub fn new(
        config: EngineConfig,
        consumer: Consumer<MarketEvent>,
        kill_switch: Arc<AtomicBool>,
        sink: S,
    ) -> Self {
        let signal_gen = SignalGenerator::new(config.signal_config.clone());
        Self {
            config,
            consumer,
            kill_switch,
            symbol_states: HashMap::new(),
            pair_states: HashMap::new(),
            signal_gen,
            sink,
            total_events: 0,
        }
    }

    /// 페어 트레이딩 대상 등록
    pub fn register_pair(&mut self, leg_a: Symbol, leg_b: Symbol, hedge_ratio: f64) {
        let key = PairKey { leg_a, leg_b };
        self.pair_states.insert(
            key,
            PairState {
                ou_model: OuModel::new(self.config.ou_config.clone()),
                hedge_ratio,
            },
        );
        tracing::info!(
            a = %leg_a, b = %leg_b, ratio = hedge_ratio,
            "registered trading pair"
        );
    }

    /// 메인 이벤트 루프 (OS 스레드에서 실행)
    ///
    /// ## 종료 조건
    /// - `kill_switch`가 활성화되면 graceful shutdown
    ///
    /// ## 성능 특성
    /// - Lock-free ring buffer pop (rtrb)
    /// - 심볼 lookup: HashMap O(1) amortized
    /// - 모든 연산 O(1) per tick (Kalman, OU update, signal gen)
    pub fn run(&mut self) {
        tracing::info!("strategy engine started");

        loop {
            if self.kill_switch.load(Ordering::Acquire) {
                break;
            }

            match self.consumer.pop() {
                Ok(event) => {
                    self.total_events += 1;
                    self.process_event(event);

                    // 주기적 지표 로깅
                    if self.total_events % self.config.metrics_log_interval == 0 {
                        self.log_metrics();
                    }
                }
                Err(_) => {
                    // 링 버퍼 비어있음 → spin loop hint
                    std::hint::spin_loop();
                }
            }
        }

        tracing::info!(
            total_events = self.total_events,
            symbols = self.symbol_states.len(),
            pairs = self.pair_states.len(),
            "strategy engine stopped"
        );
    }

    /// 단일 이벤트 처리 파이프라인
    fn process_event(&mut self, event: MarketEvent) {
        match event {
            MarketEvent::Bbo(bbo) => {
                let mid = (bbo.bid_price + bbo.ask_price) / 2.0;
                let symbol = bbo.symbol;

                // ── 심볼별 상태 업데이트 ──
                let state = self
                    .symbol_states
                    .entry(symbol)
                    .or_insert_with(|| SymbolState::new(&self.config));

                state.last_mid = mid;
                state.tick_count += 1;

                // Kalman Filter 업데이트
                let kalman_out = state.kalman.update(mid).ok();
                if let Some(ref kf) = kalman_out {
                    state.last_filtered_price = kf.estimated_price;
                }

                // Rolling stats 업데이트
                state.mid_window.push(mid);

                // EMA 업데이트
                state.ema_fast.update(mid);
                state.ema_slow.update(mid);

                // ── 페어 트레이딩: 스프레드 계산 및 OU 업데이트 ──
                self.update_pairs(symbol, bbo.local_ts_ns);

                // ── 단일 심볼 신호 생성 (Kalman 기반) ──
                // 페어가 아닌 단일 자산 전략에서도 사용 가능
                let signal = self.signal_gen.generate(
                    None, // OU는 페어에서만 사용
                    kalman_out.as_ref(),
                    bbo.local_ts_ns,
                );

                self.sink.on_signal(&symbol, &signal);
            }
            MarketEvent::Trade(trade) => {
                // Trade 이벤트는 체결 분석에 활용 (간소화: BBO만 처리)
                let state = self
                    .symbol_states
                    .entry(trade.symbol)
                    .or_insert_with(|| SymbolState::new(&self.config));
                state.tick_count += 1;
            }
            MarketEvent::Heartbeat { .. } => {
                // 연결 상태 확인용 — 무시
            }
            _ => {}
        }
    }

    /// 등록된 모든 페어의 스프레드를 업데이트하고 OU 신호 생성
    fn update_pairs(&mut self, updated_symbol: Symbol, ts_ns: u64) {
        // 성능 고려: pair_states를 순회하면서 관련 페어만 업데이트
        // (페어 수가 적으면 전체 순회도 무방)
        let pair_keys: Vec<PairKey> = self.pair_states.keys().cloned().collect();

        for key in pair_keys {
            if key.leg_a != updated_symbol && key.leg_b != updated_symbol {
                continue;
            }

            // 양쪽 다리의 최근 mid-price 가져오기
            let mid_a = self
                .symbol_states
                .get(&key.leg_a)
                .map(|s| s.last_filtered_price) // Kalman 필터링된 가격 사용
                .unwrap_or(f64::NAN);
            let mid_b = self
                .symbol_states
                .get(&key.leg_b)
                .map(|s| s.last_filtered_price)
                .unwrap_or(f64::NAN);

            if mid_a.is_nan() || mid_b.is_nan() {
                continue;
            }

            if let Some(pair_state) = self.pair_states.get_mut(&key) {
                // 스프레드 = leg_a - hedge_ratio * leg_b
                let spread = mid_a - pair_state.hedge_ratio * mid_b;

                // OU 모델 업데이트
                let ou_signal = pair_state.ou_model.update(spread);

                // 페어 트레이딩 신호 생성
                let kalman_out_a = self
                    .symbol_states
                    .get_mut(&key.leg_a)
                    .and_then(|s| s.kalman.update(s.last_mid).ok());

                let signal = self.signal_gen.generate(
                    ou_signal.as_ref(),
                    kalman_out_a.as_ref(),
                    ts_ns,
                );

                // 페어 신호 발행 (leg_a 기준)
                self.sink.on_signal(&key.leg_a, &signal);
            }
        }
    }

    fn log_metrics(&self) {
        tracing::info!(
            total_events = self.total_events,
            symbols_tracked = self.symbol_states.len(),
            pairs_tracked = self.pair_states.len(),
            "engine metrics"
        );

        // 심볼별 상세 지표
        for (symbol, state) in &self.symbol_states {
            if state.mid_window.is_ready() {
                tracing::debug!(
                    symbol = %symbol,
                    ticks = state.tick_count,
                    mid = format!("{:.2}", state.last_mid),
                    filtered = format!("{:.2}", state.last_filtered_price),
                    volatility = format!("{:.6}", state.mid_window.std_dev()),
                    ema_fast = format!("{:.2}", state.ema_fast.value()),
                    ema_slow = format!("{:.2}", state.ema_slow.value()),
                    "symbol state"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_ingestion::types::*;
    use rtrb::RingBuffer;
    use std::sync::atomic::AtomicBool;

    /// 테스트용 신호 수집기
    struct CollectorSink {
        signals: Vec<(Symbol, TradingSignal)>,
    }

    impl CollectorSink {
        fn new() -> Self {
            Self {
                signals: Vec::new(),
            }
        }
    }

    impl SignalSink for CollectorSink {
        fn on_signal(&mut self, symbol: &Symbol, signal: &TradingSignal) {
            self.signals.push((*symbol, *signal));
        }
    }

    #[test]
    fn test_engine_processes_bbo_events() {
        let (mut producer, consumer) = RingBuffer::<MarketEvent>::new(1024);
        let kill_switch = Arc::new(AtomicBool::new(false));
        let sink = CollectorSink::new();

        let mut engine = StrategyEngine::new(
            EngineConfig::default(),
            consumer,
            kill_switch.clone(),
            sink,
        );

        let sym = Symbol::from_str("BTCUSDT");

        // BBO 이벤트 50개 주입
        for i in 0..50 {
            let bid = 50000.0 + (i as f64) * 0.1;
            let ask = bid + 1.0;
            producer
                .push(MarketEvent::Bbo(BboSnapshot {
                    symbol: sym,
                    exchange: Exchange::Binance,
                    bid_price: bid,
                    bid_qty: 1.0,
                    ask_price: ask,
                    ask_qty: 1.0,
                    exchange_ts_ns: 0,
                    local_ts_ns: 0,
                }))
                .unwrap();
        }

        // 킬 스위치 예약 (50개 처리 후 종료)
        let ks = kill_switch.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            ks.store(true, Ordering::Release);
        });

        engine.run();

        assert_eq!(engine.total_events, 50);
        assert!(engine.symbol_states.contains_key(&sym));
    }
}
