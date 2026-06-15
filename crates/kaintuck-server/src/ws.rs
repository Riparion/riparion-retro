//! Per-connection WebSocket handling: translate JSON [`ClientMsg`]s into actor
//! [`Command`]s, and stream [`ServerMsg`] market updates back. One select loop
//! per socket multiplexes the broadcast feed with the client's own messages.

use axum::extract::ws::{Message, WebSocket};
use tokio::sync::{broadcast, oneshot};
use tracing::{debug, info, warn};

use kaintuck_engine::net::{ClientMsg, ServerMsg};

use crate::actor::{Command, Handle};

/// Drive one client connection to completion.
pub async fn handle_socket(mut socket: WebSocket, handle: Handle) {
    let mut events = handle.events.subscribe();
    info!("websocket connected");

    let reason = loop {
        tokio::select! {
            // Market updates fanned out by the actor.
            evt = events.recv() => {
                match evt {
                    Ok(msg) => {
                        if send(&mut socket, &msg).await.is_err() {
                            break "send failed";
                        }
                    }
                    // Fell behind the broadcast buffer — resync on the next tick.
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "client lagged the market broadcast");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break "broadcast closed",
                }
            }
            // Messages from this client.
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if handle_client_msg(&mut socket, &handle, text.as_str()).await.is_err() {
                            break "client requested leave / actor gone";
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break "client closed",
                    Some(Ok(_)) => {} // ignore binary/ping/pong
                    Some(Err(e)) => {
                        warn!(error = %e, "websocket receive error");
                        break "receive error";
                    }
                }
            }
        }
    };
    info!(reason, "websocket disconnected");
}

/// Handle one decoded client message. Returns `Err(())` if the socket should close.
async fn handle_client_msg(socket: &mut WebSocket, handle: &Handle, text: &str) -> Result<(), ()> {
    let msg: ClientMsg = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "could not decode client message");
            return send(socket, &ServerMsg::Error { message: format!("bad message: {e}") }).await;
        }
    };

    match msg {
        ClientMsg::Leave => {
            debug!("client sent Leave");
            Err(())
        }
        ClientMsg::Join { handle: _ } => {
            let (tx, rx) = oneshot::channel();
            if handle.cmd.send(Command::Join { reply: tx }).await.is_err() {
                return Err(());
            }
            match rx.await {
                Ok((participant_id, snapshot)) => {
                    send(socket, &ServerMsg::Welcome { participant_id, snapshot }).await
                }
                Err(_) => Err(()),
            }
        }
        ClientMsg::TradeOrder { town, good, side, qty } => {
            let (tx, rx) = oneshot::channel();
            let order = Command::Trade { town, good, side, qty, reply: tx };
            if handle.cmd.send(order).await.is_err() {
                return Err(());
            }
            let reply = match rx.await {
                Ok(Ok(mid)) => ServerMsg::TradeFill { town, good, side, qty, mid },
                Ok(Err(message)) => ServerMsg::Error { message },
                Err(_) => return Err(()),
            };
            send(socket, &reply).await
        }
        ClientMsg::Resync => {
            let (tx, rx) = oneshot::channel();
            if handle.cmd.send(Command::Snapshot { reply: tx }).await.is_err() {
                return Err(());
            }
            match rx.await {
                Ok(snapshot) => send(socket, &ServerMsg::Snapshot(snapshot)).await,
                Err(_) => Err(()),
            }
        }
    }
}

/// JSON-encode and send a server message; `Err(())` if the socket is gone.
async fn send(socket: &mut WebSocket, msg: &ServerMsg) -> Result<(), ()> {
    let text = serde_json::to_string(msg).map_err(|_| ())?;
    socket.send(Message::Text(text.into())).await.map_err(|_| ())
}
