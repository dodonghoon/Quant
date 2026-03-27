//! # WebSocket ハンドラ
//!
//! Axum WebSocket エンドポイント（リアルタイム市場データ、シグナル、注文、リスク、システムメトリクス）

use axum::{
    extract::{ws::{WebSocketUpgrade, WebSocket, Message}, State},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, warn, info};

use crate::{
    bridge::DashboardEvent,
    AppState,
};

use super::throttle::Throttle;

/// WebSocket market data handler — streams BBO/trade data from broadcast channel
pub async fn ws_market_data(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_market_data(socket, state))
}

async fn handle_market_data(mut socket: WebSocket, state: AppState) {
    let mut rx = state.engine_bridge.event_tx.subscribe();
    let mut throttle = Throttle::new(50); // 20 Hz max

    while let Ok(event) = rx.recv().await {
        if !throttle.should_send() {
            continue;
        }

        // Only forward SystemMetrics events to the market-data channel.
        // All other event types are handled by their own dedicated WS handlers.
        if !matches!(event, DashboardEvent::SystemMetrics(_)) {
            continue;
        }

        let json_msg = match serde_json::to_string(&event) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to serialize market data event: {}", e);
                continue;
            }
        };

        if socket.send(Message::Text(json_msg)).await.is_err() {
            info!("Client disconnected from market-data channel");
            break;
        }
    }
}

/// WebSocket signals handler — streams TradingSignal events
pub async fn ws_signals(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_signals(socket, state))
}

async fn handle_signals(mut socket: WebSocket, state: AppState) {
    let mut rx = state.engine_bridge.event_tx.subscribe();

    while let Ok(event) = rx.recv().await {
        match event {
            DashboardEvent::Signal(signal_info) => {
                let json_msg = match serde_json::to_string(&json!({
                    "channel": "signals",
                    "data": signal_info
                })) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Failed to serialize signal event: {}", e);
                        continue;
                    }
                };

                if socket.send(Message::Text(json_msg)).await.is_err() {
                    info!("Client disconnected from signals channel");
                    break;
                }
            }
            _ => continue,
        }
    }
}

/// WebSocket orders handler — streams order events (new/fill/cancel)
pub async fn ws_orders(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_orders(socket, state))
}

async fn handle_orders(mut socket: WebSocket, state: AppState) {
    let mut rx = state.engine_bridge.event_tx.subscribe();

    while let Ok(event) = rx.recv().await {
        match event {
            DashboardEvent::OrderUpdate(order_info) => {
                let json_msg = match serde_json::to_string(&json!({
                    "channel": "orders",
                    "data": order_info
                })) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Failed to serialize order event: {}", e);
                        continue;
                    }
                };

                if socket.send(Message::Text(json_msg)).await.is_err() {
                    info!("Client disconnected from orders channel");
                    break;
                }
            }
            DashboardEvent::Fill(fill_info) => {
                let json_msg = match serde_json::to_string(&json!({
                    "channel": "orders",
                    "data": fill_info
                })) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Failed to serialize fill event: {}", e);
                        continue;
                    }
                };

                if socket.send(Message::Text(json_msg)).await.is_err() {
                    info!("Client disconnected from orders channel");
                    break;
                }
            }
            _ => continue,
        }
    }
}

/// WebSocket risk handler — streams risk metrics every 1 second
pub async fn ws_risk(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_risk(socket, state))
}

async fn handle_risk(mut socket: WebSocket, state: AppState) {
    let mut rx = state.engine_bridge.event_tx.subscribe();
    let mut interval = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                if let DashboardEvent::RiskUpdate { daily_pnl, exposure, kill_switch } = event {
                    let json_msg = match serde_json::to_string(&json!({
                        "channel": "risk",
                        "data": {
                            "daily_pnl": daily_pnl,
                            "exposure": exposure,
                            "kill_switch": kill_switch
                        }
                    })) {
                        Ok(msg) => msg,
                        Err(e) => {
                            error!("Failed to serialize risk event: {}", e);
                            continue;
                        }
                    };

                    if socket.send(Message::Text(json_msg)).await.is_err() {
                        info!("Client disconnected from risk channel");
                        return;
                    }
                }
            }
            _ = interval.tick() => {
                // Periodic tick for risk metric sampling
                // The event stream will provide the latest risk update
            }
        }
    }
}

/// WebSocket system handler — streams system metrics every 5 seconds
pub async fn ws_system(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_system(socket, state))
}

async fn handle_system(mut socket: WebSocket, state: AppState) {
    let mut rx = state.engine_bridge.event_tx.subscribe();
    let mut interval = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                if let DashboardEvent::SystemMetrics(system_state) = event {
                    let json_msg = match serde_json::to_string(&json!({
                        "channel": "system",
                        "data": system_state
                    })) {
                        Ok(msg) => msg,
                        Err(e) => {
                            error!("Failed to serialize system event: {}", e);
                            continue;
                        }
                    };

                    if socket.send(Message::Text(json_msg)).await.is_err() {
                        info!("Client disconnected from system channel");
                        return;
                    }
                }
            }
            _ = interval.tick() => {
                // Periodic tick for system metric sampling
                // The event stream will provide the latest system metrics
            }
        }
    }
}

/// WebSocket models handler — streams model state every 1 second
pub async fn ws_models(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_models(socket, state))
}

async fn handle_models(mut socket: WebSocket, state: AppState) {
    let mut _rx = state.engine_bridge.event_tx.subscribe();
    let mut interval = interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        // TODO: Fetch latest model state from engine_bridge snapshot
        // For now, send a placeholder with current timestamp
        let json_msg = match serde_json::to_string(&json!({
            "channel": "models",
            "data": {
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "status": "monitoring"
            }
        })) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to serialize model state: {}", e);
                continue;
            }
        };

        if socket.send(Message::Text(json_msg)).await.is_err() {
            info!("Client disconnected from models channel");
            return;
        }
    }
}
