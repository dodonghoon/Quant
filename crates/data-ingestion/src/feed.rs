//! # Upbit WebSocket Feed — Live Data Ingestion
//!
//! wss://api.upbit.com/websocket/v1 에 연결하여 "ticker" 타입 메시지를 수신,
//! Redis와 QuestDB에 실시간 시장 데이터를 기록합니다.
//!
//! ## 저장 대상
//! | 저장소    | 키 / 테이블             | 목적                              |
//! |----------|------------------------|-----------------------------------|
//! | Redis    | `quant:market_data`    | Python LLM 엔진 GET (TTL 300s)    |
//! | Redis    | `ticks:live`           | Pub/Sub 실시간 구독자             |
//! | QuestDB  | `upbit_tickers`        | 시계열 틱 영구 저장 (ML 연구)     |

use crate::questdb::QuestDbWriter;
use crate::redis_store::RedisTickStore;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use uuid::Uuid;

/// 알트코인 심볼 목록 (BTC/ETH 제외)
const ALTCOIN_CODES: &[&str] = &[
    "KRW-XRP", "KRW-SOL", "KRW-ADA", "KRW-DOGE",
    "KRW-AVAX", "KRW-LINK", "KRW-DOT", "KRW-ATOM",
];

/// QuestDB 플러시 간격 (틱 수 기준)
/// BufWriter(64KB)에 여유 있게, 너무 자주 syscall 하지 않도록 50 틱마다 플러시
const QDB_FLUSH_EVERY: u64 = 50;

pub async fn start_upbit_websocket() {
    // 환경 변수 로드
    dotenvy::from_filename("config/.env.production").ok();

    let redis_url = env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let questdb_host = env::var("QUESTDB_HOST")
        .unwrap_or_else(|_| "localhost".to_string());
    let questdb_port: u16 = env::var("QUESTDB_ILP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9009);

    // ── Redis 연결 ────────────────────────────────────────────────────────────
    let store = match RedisTickStore::new(&redis_url).await {
        Ok(s) => {
            info!("[FEED] Redis connected: {}", redis_url);
            s
        }
        Err(e) => {
            error!("[FEED] Redis connection failed: {}. Aborting feed.", e);
            return;
        }
    };

    // ── QuestDB 연결 (선택적 — 실패해도 Redis/WebSocket 계속 동작) ────────────
    // QuestDbWriter는 동기 blocking TCP이므로 tokio::task::block_in_place로 호출
    let mut qdb_writer: Option<QuestDbWriter> =
        match QuestDbWriter::new(&questdb_host, questdb_port) {
            Ok(w) => {
                info!(
                    "[FEED] QuestDB connected: {}:{}  (table: upbit_tickers)",
                    questdb_host, questdb_port
                );
                Some(w)
            }
            Err(e) => {
                warn!(
                    "[FEED] QuestDB unavailable ({}:{}): {}. \
                     Tick persistence disabled — run setup_questdb.ps1 first.",
                    questdb_host, questdb_port, e
                );
                None
            }
        };

    // ── Upbit WebSocket 연결 ──────────────────────────────────────────────────
    let url = "wss://api.upbit.com/websocket/v1";

    let (ws_stream, _) = match connect_async(url).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("[FEED] Failed to connect to Upbit WebSocket: {}", e);
            return;
        }
    };

    info!("[FEED] Connected to Upbit WebSocket");
    let (mut write, mut read) = ws_stream.split();

    // ── 구독 메시지 ───────────────────────────────────────────────────────────
    let ticket = Uuid::new_v4().to_string();
    let subscribe_msg = json!([
        { "ticket": ticket },
        {
            "type": "ticker",
            "codes": [
                "KRW-BTC", "KRW-ETH",
                "KRW-XRP", "KRW-SOL", "KRW-ADA", "KRW-DOGE",
                "KRW-AVAX", "KRW-LINK", "KRW-DOT", "KRW-ATOM"
            ]
        },
        { "format": "DEFAULT" }
    ]);

    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .expect("[FEED] Failed to send subscription message");

    info!("[FEED] Subscribed to 10 KRW-* ticker streams");

    // ── 상태 변수 ─────────────────────────────────────────────────────────────
    let mut change_rates: HashMap<String, f64> = HashMap::new();
    let mut tick_count: u64 = 0;

    // ── 메시지 루프 ───────────────────────────────────────────────────────────
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                process_message(
                    &data, &store, &mut change_rates,
                    &mut qdb_writer, &mut tick_count,
                )
                .await;
            }
            Ok(Message::Text(text)) => {
                process_message(
                    text.as_bytes(), &store, &mut change_rates,
                    &mut qdb_writer, &mut tick_count,
                )
                .await;
            }
            Ok(Message::Ping(ping)) => {
                let _ = write.send(Message::Pong(ping)).await;
            }
            Ok(Message::Close(_)) => {
                warn!("[FEED] WebSocket closed by Upbit server.");
                break;
            }
            Err(e) => {
                error!("[FEED] WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // ── 종료 시 QuestDB 남은 버퍼 플러시 ─────────────────────────────────────
    if let Some(ref mut writer) = qdb_writer {
        tokio::task::block_in_place(|| {
            if let Err(e) = writer.flush() {
                warn!("[QDB] Final flush failed: {}", e);
            } else {
                info!("[QDB] Final flush on shutdown. Total ticks written: {}", tick_count);
            }
        });
    }

    info!("[FEED] Upbit WebSocket feed exited. Total ticks processed: {}", tick_count);
}

/// 단일 WebSocket 메시지를 처리하여 Redis + QuestDB에 기록
async fn process_message(
    data: &[u8],
    store: &RedisTickStore,
    change_rates: &mut HashMap<String, f64>,
    qdb_writer: &mut Option<QuestDbWriter>,
    tick_count: &mut u64,
) {
    let parsed = match serde_json::from_slice::<serde_json::Value>(data) {
        Ok(v) => v,
        Err(_) => return,
    };

    if parsed.get("type").and_then(|v| v.as_str()) != Some("ticker") {
        return;
    }

    let code = match parsed.get("code").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => return,
    };

    let change_rate = parsed
        .get("signed_change_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let trade_price = parsed
        .get("trade_price")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Upbit ticker에서 추가 필드 추출 (QuestDB 기록용)
    let trade_volume = parsed
        .get("trade_volume")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let ask_bid = parsed
        .get("ask_bid")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");

    // 거래소 타임스탬프 (ms → ns)  ← 로컬 시계보다 정확
    let exchange_ts_ns = parsed
        .get("timestamp")
        .and_then(|v| v.as_u64())
        .map(|ms| ms * 1_000_000)
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        });

    let pct = change_rate * 100.0;
    change_rates.insert(code.clone(), pct);

    debug!(
        "[TICK] {}  price={:.0} KRW  24h={:+.2}%  vol={:.6}  side={}  (tracked: {})",
        code, trade_price, pct, trade_volume, ask_bid, change_rates.len()
    );

    // ── QuestDB 틱 기록 (영구 시계열 저장) ───────────────────────────────────
    // block_in_place: tokio multi-thread 런타임에서 동기 IO를 안전하게 실행
    *tick_count += 1;
    let should_flush = *tick_count % QDB_FLUSH_EVERY == 0;
    let tc = *tick_count;

    if let Some(ref mut writer) = qdb_writer {
        tokio::task::block_in_place(|| {
            match writer.write_ticker(&code, exchange_ts_ns, trade_price, trade_volume, pct, ask_bid) {
                Ok(_) => {}
                Err(e) => {
                    warn!("[QDB] write_ticker failed for {}: {}", code, e);
                }
            }

            if should_flush {
                match writer.flush() {
                    Ok(_) => debug!("[QDB] Flushed after {} ticks", tc),
                    Err(e) => warn!("[QDB] Flush failed at tick {}: {}", tc, e),
                }
            }
        });
    }

    // ── Redis: 개별 틱 Pub/Sub ────────────────────────────────────────────────
    if let Ok(tick_bytes) = serde_json::to_vec(&json!({
        "code":            code,
        "price":           trade_price,
        "volume":          trade_volume,
        "change_pct_24h":  pct,
        "side":            ask_bid,
        "ts_ns":           exchange_ts_ns,
    })) {
        let _ = store.publish_event("ticks:live", tick_bytes).await;
    }

    // ── Redis: 집계 컨텍스트 SET (BTC 수신 후부터) ────────────────────────────
    if !change_rates.contains_key("KRW-BTC") {
        return;
    }

    let btc_pct = change_rates["KRW-BTC"];
    let eth_pct = change_rates.get("KRW-ETH").copied().unwrap_or(0.0);

    let alt_values: Vec<f64> = ALTCOIN_CODES
        .iter()
        .filter_map(|s| change_rates.get(*s).copied())
        .collect();

    let altcoin_avg = if alt_values.is_empty() {
        0.0
    } else {
        alt_values.iter().sum::<f64>() / alt_values.len() as f64
    };

    let divergence = altcoin_avg - btc_pct;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let context = json!({
        "btc_7d_pct":         btc_pct,
        "eth_7d_pct":         eth_pct,
        "altcoin_avg_7d_pct": altcoin_avg,
        "divergence_pct":     divergence,
        "data_source":        "live_redis",
        "note":               "24h signed_change_rate (Upbit WebSocket)",
        "symbols_tracked":    change_rates.len(),
        "updated_at":         now_secs,
    });

    let ctx_str = context.to_string();

    match store.set_market_context(&ctx_str).await {
        Ok(_) => debug!(
            "[FEED] SET quant:market_data  btc={:+.2}%  eth={:+.2}%  \
             alt={:+.2}%  div={:+.2}%  ticks={}",
            btc_pct, eth_pct, altcoin_avg, divergence, tc
        ),
        Err(e) => warn!("[FEED] SET quant:market_data failed: {}", e),
    }

    if let Ok(ctx_bytes) = serde_json::to_vec(&context) {
        let _ = store.publish_event("quant:market_data_update", ctx_bytes).await;
    }
}
