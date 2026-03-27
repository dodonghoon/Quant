//! # WebSocket Feed Handler
//!
//! 거래소 WebSocket에 연결하여 실시간 데이터를 수신하고,
//! 파서를 거쳐 정규화된 `MarketEvent`를 SPSC 링 버퍼로 전달합니다.
//!
//! ## 아키텍처
//! ```text
//!   Exchange WS ──→ [FeedHandler] ──→ rtrb::Producer<MarketEvent>
//!                      │                        │
//!                   (parse)              (lock-free push)
//!                      │                        │
//!                   tokio task            Strategy Engine이
//!                                        Consumer로 읽음
//! ```
//!
//! ## 재연결 전략
//! 지수 백오프(Exponential Backoff) + 최대 30초 캡

use crate::error::{IngestionError, Result};
use crate::parser::ExchangeParser;
use crate::types::MarketEvent;
use futures_util::{SinkExt, StreamExt};
use rtrb::Producer;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

// ────────────────────────────────────────────
// Configuration
// ────────────────────────────────────────────

pub struct FeedConfig {
    /// WebSocket URL (e.g., "wss://stream.binance.com:9443/stream")
    pub ws_url: String,
    /// 구독할 심볼 목록
    pub symbols: Vec<String>,
    /// 재연결 시도 최대 횟수 (0 = 무제한)
    pub max_reconnect_attempts: u32,
    /// 초기 재연결 대기 시간 (밀리초)
    pub initial_backoff_ms: u64,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            ws_url: "wss://stream.binance.com:9443/stream".into(),
            symbols: vec!["btcusdt".into()],
            max_reconnect_attempts: 0, // 무제한
            initial_backoff_ms: 100,
        }
    }
}

// ────────────────────────────────────────────
// Metrics (Atomic, Lock-free)
// ────────────────────────────────────────────

/// 실시간 처리량 및 지연 모니터링 지표
/// 모든 필드가 AtomicU64 → 락 없이 읽기/쓰기 가능
pub struct FeedMetrics {
    pub messages_received: AtomicU64,
    pub events_published: AtomicU64,
    pub parse_errors: AtomicU64,
    pub buffer_drops: AtomicU64,
    pub reconnect_count: AtomicU64,
}

impl FeedMetrics {
    pub fn new() -> Self {
        Self {
            messages_received: AtomicU64::new(0),
            events_published: AtomicU64::new(0),
            parse_errors: AtomicU64::new(0),
            buffer_drops: AtomicU64::new(0),
            reconnect_count: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            messages_received: self.messages_received.load(Ordering::Relaxed),
            events_published: self.events_published.load(Ordering::Relaxed),
            parse_errors: self.parse_errors.load(Ordering::Relaxed),
            buffer_drops: self.buffer_drops.load(Ordering::Relaxed),
            reconnect_count: self.reconnect_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub messages_received: u64,
    pub events_published: u64,
    pub parse_errors: u64,
    pub buffer_drops: u64,
    pub reconnect_count: u64,
}

// ────────────────────────────────────────────
// FeedHandler
// ────────────────────────────────────────────

pub struct FeedHandler<P: ExchangeParser> {
    config: FeedConfig,
    parser: P,
    /// 링 버퍼 Producer (SPSC, lock-free)
    producer: Producer<MarketEvent>,
    /// 전역 킬 스위치 (AtomicBool, 다른 모듈과 공유)
    kill_switch: Arc<AtomicBool>,
    /// 실시간 지표
    metrics: Arc<FeedMetrics>,
}

impl<P: ExchangeParser> FeedHandler<P> {
    pub fn new(
        config: FeedConfig,
        parser: P,
        producer: Producer<MarketEvent>,
        kill_switch: Arc<AtomicBool>,
        metrics: Arc<FeedMetrics>,
    ) -> Self {
        Self {
            config,
            parser,
            producer,
            kill_switch,
            metrics,
        }
    }

    /// 메인 이벤트 루프 — 연결, 구독, 수신, 재연결을 모두 처리
    pub async fn run(&mut self) -> Result<()> {
        let mut attempt: u32 = 0;
        let mut backoff_ms = self.config.initial_backoff_ms;

        loop {
            // ── 킬 스위치 확인 ──
            if self.kill_switch.load(Ordering::Acquire) {
                tracing::warn!("kill switch activated, shutting down feed handler");
                return Ok(());
            }

            match self.connect_and_stream().await {
                Ok(()) => {
                    // 정상 종료 (킬 스위치 등)
                    tracing::info!("feed handler exited gracefully");
                    return Ok(());
                }
                Err(e) => {
                    attempt += 1;
                    self.metrics.reconnect_count.fetch_add(1, Ordering::Relaxed);

                    tracing::error!(
                        error = %e,
                        attempt = attempt,
                        backoff_ms = backoff_ms,
                        "connection lost, reconnecting..."
                    );

                    // 최대 시도 횟수 확인
                    if self.config.max_reconnect_attempts > 0
                        && attempt >= self.config.max_reconnect_attempts
                    {
                        return Err(IngestionError::ConfigError(format!(
                            "max reconnect attempts ({}) exceeded",
                            self.config.max_reconnect_attempts
                        )));
                    }

                    // 지수 백오프 대기 (최대 30초)
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(30_000);
                }
            }
        }
    }

    /// 단일 연결 세션: 연결 → 구독 → 수신 루프
    async fn connect_and_stream(&mut self) -> Result<()> {
        let url = url::Url::parse(&self.config.ws_url).map_err(|e| {
            IngestionError::ConfigError(format!("invalid WebSocket URL: {e}"))
        })?;

        tracing::info!(url = %url, "connecting to exchange WebSocket...");

        let (ws_stream, _response) =
            connect_async(url.as_str())
                .await
                .map_err(|e| IngestionError::ConnectionFailed {
                    url: self.config.ws_url.clone(),
                    source: e,
                })?;

        let (mut write, mut read) = ws_stream.split();

        // ── 구독 메시지 전송 ──
        let symbol_refs: Vec<&str> = self.config.symbols.iter().map(|s| s.as_str()).collect();
        let sub_msg = self.parser.subscription_message(&symbol_refs);

        tracing::info!(symbols = ?self.config.symbols, "subscribing to streams");
        write
            .send(Message::Text(sub_msg))
            .await
            .map_err(IngestionError::ReceiveError)?;

        // ── 수신 루프 ──
        while let Some(msg_result) = read.next().await {
            // 킬 스위치 확인 (매 메시지마다 — 오버헤드 무시할 수준)
            if self.kill_switch.load(Ordering::Acquire) {
                tracing::warn!("kill switch activated during stream");
                return Ok(());
            }

            let msg = msg_result.map_err(IngestionError::ReceiveError)?;
            self.metrics
                .messages_received
                .fetch_add(1, Ordering::Relaxed);

            match msg {
                Message::Text(text) => {
                    self.handle_raw_message(text.as_bytes())?;
                }
                Message::Binary(data) => {
                    self.handle_raw_message(&data)?;
                }
                Message::Ping(payload) => {
                    // Pong 자동 응답 (tungstenite가 처리하지만 명시적으로도)
                    let _ = write.send(Message::Pong(payload)).await;
                }
                Message::Close(_) => {
                    tracing::warn!("server initiated close");
                    return Err(IngestionError::StreamDisconnected);
                }
                _ => {} // Pong, Frame 등은 무시
            }
        }

        // 스트림 소진 = 연결 끊김
        Err(IngestionError::StreamDisconnected)
    }

    /// 원장 바이트 → 파싱 → 링 버퍼 push
    #[inline]
    fn handle_raw_message(&mut self, raw: &[u8]) -> Result<()> {
        match self.parser.parse(raw) {
            Ok(Some(event)) => {
                // rtrb::Producer::push: lock-free, 실패 시 버퍼 풀
                match self.producer.push(event) {
                    Ok(()) => {
                        self.metrics
                            .events_published
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        // 버퍼 가득 참 — 이벤트 드롭 (로깅만, 크래시 아님)
                        self.metrics.buffer_drops.fetch_add(1, Ordering::Relaxed);
                        tracing::warn!("ring buffer full, dropped event");
                    }
                }
            }
            Ok(None) => {
                // 무시할 메시지 (heartbeat, 미지원 스트림 등)
            }
            Err(e) => {
                self.metrics.parse_errors.fetch_add(1, Ordering::Relaxed);
                tracing::debug!(error = %e, "parse error (non-fatal)");
            }
        }
        Ok(())
    }
}
