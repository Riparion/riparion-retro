//! Library surface of `kaintuck-server`: the world actor, the WebSocket
//! connection handler, and the axum router that ties them together, so both the
//! `main` binary and integration tests can stand the server up.

pub mod actor;
pub mod ws;

use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::any;
use axum::Router;

use actor::Handle;

/// Build the axum app over an actor [`Handle`]: a `/ws` upgrade endpoint and a
/// `/health` probe.
pub fn app(handle: Handle) -> Router {
    Router::new()
        .route("/ws", any(ws_route))
        .route("/health", any(|| async { "ok" }))
        .with_state(handle)
}

async fn ws_route(ws: WebSocketUpgrade, State(handle): State<Handle>) -> Response {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, handle))
}
