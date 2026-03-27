//! # Data Ingestion 실행 예제
//!
//! 전체 파이프라인 시연:
//! 1. rtrb 링 버퍼 생성 (64K 슬롯)
//! 2. FeedHandler (Producer) — tokio 태스크에서 WS 수신
//! 3. Consumer — 별도 OS 스레드에서 이벤트 처리 (CPU-bound, tokio 블로킹 방지)
//!
//! ```text
//!   [tokio task]                    [OS thread]
//!   FeedHandler ──rtrb──→ Consumer (Strategy Engine으로 전달)
//!   (I/O bound)           (CPU bound, tokio 런타임 밖)
//! ```

use data_ingestion::*;
use rtrb::RingBuffer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    // ── 로깅 초기화 ──
    tracing_subscriber::fmt()
        .with_env_filter("data_ingestion=debug,info")
        .with_target(false)
        .init();

    // ── 공유 상태 ──
    let kill_switch = Arc::new(AtomicBool::new(false));
    let metrics = Arc::new(FeedMetrics::new());

    // ── SPSC 링 버퍼: 64K 슬롯 (MarketEvent ~128B × 64K ≈ 8MB) ──
    let (producer, mut consumer) = RingBuffer::<MarketEvent>::new(65_536);

    // ── Ctrl+C 핸들러 ──
    let ks = kill_switch.clone();
    ctrlc_handler(ks);

    // ── Consumer: 전용 OS 스레드 (CPU-bound 작업은 tokio 밖에서) ──
    let ks_consumer = kill_switch.clone();
    let metrics_consumer = metrics.clone();

    let consumer_handle = std::thread::Builder::new()
        .name("event-consumer".into())
        .spawn(move || {
            tracing::info!("consumer thread started");
            let mut event_count: u64 = 0;

            loop {
                if ks_consumer.load(Ordering::Acquire) {
                    break;
                }

                // Non-blocking pop: 링 버퍼에서 이벤트 꺼내기
                match consumer.pop() {
                    Ok(event) => {
                        event_count += 1;

                        // 100만 이벤트마다 지연 시간 + 처리량 리포트
                        if event_count % 1_000_000 == 0 {
                            let snap = metrics_consumer.snapshot();
                            tracing::info!(
                                events = event_count,
                                published = snap.events_published,
                                drops = snap.buffer_drops,
                                parse_errors = snap.parse_errors,
                                "consumer progress"
                            );
                        }

                        // ── 여기서 Strategy Engine으로 전달 ──
                        // 예: strategy_engine.on_event(event);
                        process_event(&event);
                    }
                    Err(_) => {
                        // 버퍼 비어있음 → busy-wait 대신 짧은 yield
                        // HFT에서는 spin-wait이 좋지만 데모에서는 CPU 절약
                        std::hint::spin_loop();
                    }
                }
            }

            tracing::info!(total_events = event_count, "consumer thread exiting");
        })
        .expect("failed to spawn consumer thread");

    // ── Producer: tokio 런타임에서 FeedHandler 실행 ──
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    rt.block_on(async {
        let config = FeedConfig {
            ws_url: "wss://stream.binance.com:9443/stream".into(),
            symbols: vec!["btcusdt".into(), "ethusdt".into()],
            max_reconnect_attempts: 0, // 무제한 재연결
            initial_backoff_ms: 200,
        };

        let mut handler = FeedHandler::new(
            config,
            BinanceParser,
            producer,
            kill_switch.clone(),
            metrics.clone(),
        );

        if let Err(e) = handler.run().await {
            tracing::error!(error = %e, "feed handler terminated with error");
        }
    });

    // ── 정리 ──
    consumer_handle.join().expect("consumer thread panicked");

    let final_snap = metrics.snapshot();
    tracing::info!(
        received = final_snap.messages_received,
        published = final_snap.events_published,
        drops = final_snap.buffer_drops,
        errors = final_snap.parse_errors,
        reconnects = final_snap.reconnect_count,
        "=== final metrics ==="
    );
}

/// 이벤트 처리 스텁 — 실제로는 Strategy Engine으로 라우팅
#[inline]
fn process_event(event: &MarketEvent) {
    match event {
        MarketEvent::Trade(t) => {
            tracing::trace!(
                symbol = %t.symbol,
                price = t.price,
                qty = t.quantity,
                side = ?t.aggressor_side,
                "trade"
            );
        }
        MarketEvent::Bbo(bbo) => {
            tracing::trace!(
                symbol = %bbo.symbol,
                bid = bbo.bid_price,
                ask = bbo.ask_price,
                spread_bps = ((bbo.ask_price - bbo.bid_price) / bbo.bid_price * 10_000.0),
                "bbo update"
            );
        }
        _ => {}
    }
}

/// Ctrl+C 시그널로 graceful shutdown
fn ctrlc_handler(kill_switch: Arc<AtomicBool>) {
    // 참고: 프로덕션에서는 tokio::signal 사용 권장
    std::thread::spawn(move || {
        let mut signals_received = 0;
        loop {
            // 간단한 폴링 방식 (예제용)
            std::thread::sleep(std::time::Duration::from_millis(100));

            // 실제로는 signal hook 라이브러리 사용
            // 여기서는 kill_switch가 외부에서 설정되는 것을 가정
            if kill_switch.load(Ordering::Relaxed) {
                signals_received += 1;
                if signals_received >= 2 {
                    tracing::error!("forced shutdown");
                    std::process::exit(1);
                }
            }
        }
    });
}
