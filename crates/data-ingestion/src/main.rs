//! # Data Ingestion — Upbit WebSocket Feed
//!
//! Upbit WebSocket(`wss://api.upbit.com/websocket/v1`)에 연결하여
//! 실시간 시세를 수신하고 Redis Stream에 저장합니다.

use data_ingestion::start_upbit_websocket;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("data_ingestion=debug,info")
        .with_target(false)
        .init();

    tracing::info!("Starting Upbit WebSocket feed...");
    start_upbit_websocket().await;
    tracing::info!("Feed exited.");
}
