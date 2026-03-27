//! # Redis Signal Bridge
//!
//! Python 앙상블 엔진(quant:execution_signals)으로부터
//! 실행 신호를 수신하여 Gateway로 라우팅합니다.
//!
//! ## 채널 프로토콜
//! Python `signal_bridge.py`가 `quant:execution_signals`에 발행하는
//! JSON 페이로드를 수신합니다.
//!
//! ```json
//! {
//!   "symbol":  "KRW-XRP",
//!   "signal":  0.72,
//!   "regime":  "altseason"
//! }
//! ```
//!
//! - `signal` ∈ [-1.0, 1.0]: 양수 → 매수(bid), 음수 → 매도(ask)
//! - |signal| < 0.10 이하는 무시 (MIN_SIGNAL_THRESHOLD)

use futures_util::StreamExt;
use log::{error, info, warn};
use serde::Deserialize;

use crate::gateway::{Gateway, Order};

/// quant:execution_signals 채널 이름 (Python 쪽과 반드시 일치)
pub const SIGNAL_CHANNEL: &str = "quant:execution_signals";

/// 최소 신호 강도 — 이보다 약한 신호는 주문 생성 안 함
const MIN_SIGNAL_THRESHOLD: f64 = 0.10;

/// Python에서 발행하는 신호 페이로드
#[derive(Debug, Deserialize)]
struct SignalPayload {
    symbol: String,
    signal: f64,
    #[serde(default)]
    regime: String,
}

/// Redis Pub/Sub 구독 루프 실행
///
/// 프로세스 종료 시까지 블로킹됩니다.
/// 별도 tokio 태스크로 spawn하여 사용하세요.
pub async fn run_signal_listener(redis_url: &str) {
    info!("Redis bridge starting — subscribing to '{}'", SIGNAL_CHANNEL);

    let client = match redis::Client::open(redis_url) {
        Ok(c) => c,
        Err(e) => {
            error!("Redis client creation failed: {}", e);
            return;
        }
    };

    let mut pubsub = match client.get_async_pubsub().await {
        Ok(ps) => ps,
        Err(e) => {
            error!("Redis pubsub connection failed: {}", e);
            return;
        }
    };

    if let Err(e) = pubsub.subscribe(SIGNAL_CHANNEL).await {
        error!("Redis subscribe failed: {}", e);
        return;
    }

    info!("Listening on channel '{}'", SIGNAL_CHANNEL);
    let gateway = Gateway::new();
    let mut stream = pubsub.on_message();

    while let Some(msg) = stream.next().await {
        let raw: String = match msg.get_payload::<String>() {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to decode Redis message: {}", e);
                continue;
            }
        };

        let payload: SignalPayload = match serde_json::from_str(&raw) {
            Ok(p) => p,
            Err(e) => {
                warn!("Signal JSON parse error: {} | raw='{}'", e, raw);
                continue;
            }
        };

        // 약한 신호 필터
        if payload.signal.abs() < MIN_SIGNAL_THRESHOLD {
            info!(
                "[BRIDGE] Signal too weak ({:.3}), skipping {}",
                payload.signal, payload.symbol
            );
            continue;
        }

        let side = if payload.signal > 0.0 { "bid" } else { "ask" }.to_string();
        // 신호 강도를 최소 주문 수량(0.0001 ~ 0.01 단위)으로 변환
        let volume = format!("{:.8}", payload.signal.abs() * 0.01);

        let order = Order {
            market: payload.symbol.clone(),
            side: side.clone(),
            volume: Some(volume),
            price: None,
            ord_type: "market".to_string(),
        };

        info!(
            "[BRIDGE] Routing signal | symbol={} signal={:.3} regime={} side={}",
            payload.symbol, payload.signal, payload.regime, side
        );

        match gateway.send_order(&order).await {
            Ok(resp) => info!(
                "[BRIDGE] Order result | uuid={:?} state={:?}",
                resp.uuid, resp.state
            ),
            Err(e) => error!("[BRIDGE] Order failed: {}", e),
        }
    }

    warn!("Redis bridge exited");
}
