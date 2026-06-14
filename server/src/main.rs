//! `kaintuck-server` — a native, authoritative game server for kaintuck's shared
//! market economy.
//!
//! It runs the same headless `kaintuck-engine` the UI runs: a pool of bot traders
//! plays continuously against one shared [`World`](kaintuck_engine::world::World)
//! market, and human players connect over WebSocket to trade in the same market
//! ("independent journeys, shared market" — humans run their own journey in the
//! browser and route only their port trades here).
//!
//! ```text
//! BOTS=16 PORT=4317 TICK_MS=500 cargo run -p kaintuck-server
//! ```
//! then connect a WebSocket client to `ws://127.0.0.1:4317/ws`.

use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

use kaintuck_server::{actor, app};

#[tokio::main]
async fn main() {
    // `RUST_LOG` controls verbosity (e.g. `RUST_LOG=kaintuck_server=debug`).
    // Default: info for our crate, warn for everything else.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("kaintuck_server=info,warn")),
        )
        .init();

    let bots = env_usize("BOTS", 12);
    let port = env_usize("PORT", 4317) as u16;
    let tick_ms = env_usize("TICK_MS", 500) as u64;

    let handle = actor::spawn(bots, tick_ms);
    let app = app(handle);

    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("cannot bind {addr}: {e}"));
    info!(bots, tick_ms, %addr, "kaintuck-server listening on ws://{addr}/ws");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutting down");
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
