use log::{error, info};

/// Execution Engine — standalone binary entry point.
///
/// Loads config, verifies Redis connectivity, then runs the signal listener
/// loop. Subscribes to `quant:execution_signals` (published by
/// llm_regime_engine.py) and forwards each signal to the Upbit gateway.
#[tokio::main]
async fn main() {
    // Load environment variables from config/.env.production
    dotenvy::from_filename("config/.env.production").ok();

    // Initialize logger (RUST_LOG=info by default)
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let trading_mode = std::env::var("TRADING_MODE")
        .unwrap_or_else(|_| "paper".to_string());

    // ── Health-check: verify Redis is reachable before entering the loop ──
    info!("Checking Redis connectivity: {}", redis_url);
    match redis::Client::open(redis_url.as_str()) {
        Ok(client) => match client.get_connection() {
            Ok(mut conn) => {
                match redis::cmd("PING").query::<String>(&mut conn) {
                    Ok(pong) => info!("[HEALTH] Redis PING → {}", pong),
                    Err(e)   => error!("[HEALTH] Redis PING failed: {}", e),
                }
            }
            Err(e) => error!("[HEALTH] Redis connection failed: {} — listener may not start", e),
        },
        Err(e) => error!("[HEALTH] Redis client creation failed: {}", e),
    }

    info!("=======================================================");
    info!("[SERVICE_STARTED] EXECUTION ENGINE");
    info!(" TRADING_MODE : {}", trading_mode);
    info!(" REDIS        : {}", redis_url);
    info!(" CHANNEL      : {}", execution_engine::redis_bridge::SIGNAL_CHANNEL);
    info!(" MIN_ORDER_KRW: 5000 (Upbit minimum — enforced in gateway)");
    info!("=======================================================");

    execution_engine::redis_bridge::run_signal_listener(&redis_url).await;
}
