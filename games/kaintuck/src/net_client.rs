//! Optional multiplayer: a WebSocket client that joins the shared-market game
//! server and trades the player's port orders into the one global market.
//!
//! This is strictly opt-in and additive. It activates only when the bundle is
//! served with a `<meta name="riparion-ws-base">` tag naming the server's WS URL
//! (e.g. `ws://host:4317/ws`); absent that — plain `dx serve`, no server — every
//! hook here is a no-op and the game is byte-for-byte the offline single-player
//! experience. The model is *independent journeys, shared market*: the player's
//! whole journey runs locally in this UI, and only their river-port trades cross
//! to the server, where they move prices for everyone.
//!
//! How it threads into the engine: while connected and afloat, the player's
//! `Game` carries a [`MarketLink`] whose quote is the server's live mids for the
//! current town, so `buy_price`/`sell_price` price against the shared market. Each
//! `buy`/`sell` records a [`TradeIntent`] on the link; this client drains those
//! and forwards them as [`ClientMsg::TradeOrder`]s, and applies the server's
//! [`ServerMsg`] updates back onto the link's quote.

use dioxus::prelude::*;
use futures::future::FutureExt;
use futures::stream::StreamExt;
use futures::SinkExt;
use gloo_net::websocket::{futures::WebSocket, Message};

use crate::engine::gossip::{GossipEvent, GossipFeed};
use crate::engine::market_link::{MarketLink, QuoteSnapshot};
use crate::engine::net::{ClientMsg, ServerMsg};
use crate::engine::state::{Phase, NUM_GOODS};
use crate::engine::Game;

/// How often (ms) the client flushes pending trades even if the server hasn't
/// sent anything — so a quiet server can't strand a player's orders.
const FLUSH_INTERVAL_MS: u32 = 250;

/// The shared market as this client currently knows it, mirrored into a Dioxus
/// signal so the UI can show "connected" state and live quotes.
#[derive(Clone, PartialEq, Default)]
pub struct RemoteMarket {
    pub connected: bool,
    pub participant_id: Option<u64>,
    /// `mids[town][good]` from the server.
    pub mids: Vec<[f64; NUM_GOODS]>,
}

/// The WebSocket endpoint, from the server-injected meta tag. `None` → run
/// offline (no server configured).
pub fn ws_url() -> Option<String> {
    retro_kit::meta_content("riparion-ws-base")
}

/// Apply one decoded server message to local state, mutating in place (no
/// whole-market clone — `MarketDelta` arrives every tick).
fn apply_server_msg(remote: &mut Signal<Option<RemoteMarket>>, msg: ServerMsg) {
    let mut guard = remote.write();
    let rm = guard.get_or_insert_with(RemoteMarket::default);
    match msg {
        ServerMsg::Welcome { participant_id, snapshot } => {
            rm.connected = true;
            rm.participant_id = Some(participant_id);
            rm.mids = snapshot.mids;
        }
        ServerMsg::Snapshot(snapshot) => {
            rm.connected = true;
            rm.mids = snapshot.mids;
        }
        ServerMsg::MarketDelta { town, mids } => {
            rm.connected = true;
            if town < rm.mids.len() {
                rm.mids[town] = mids;
            }
            // A delta before any snapshot is dropped here, but the server
            // re-broadcasts every town each tick, so the next tick fills it in.
        }
        ServerMsg::TradeFill { .. } => {
            // The local engine already applied the trade against the link quote;
            // the fill is confirmation. Nothing to do for the minimal client.
            rm.connected = true;
        }
        ServerMsg::Error { .. } => {}
        // Gossip is routed to the engine's feed before this fn (see the reader
        // loop); never reaches here.
        ServerMsg::Gossip(_) => {}
    }
}

/// The current town's shared mids, if known. Reads a single row — no clone of the
/// whole mids vector.
fn current_row(remote: &Signal<Option<RemoteMarket>>, town: usize) -> Option<[f64; NUM_GOODS]> {
    remote.read().as_ref().and_then(|rm| rm.mids.get(town).copied())
}

/// Start (once) the shared-market connection, if a server is configured. Reads
/// the `Game` and `RemoteMarket` from context. Call this unconditionally from the
/// app root — it no-ops when no `riparion-ws-base` meta is present.
pub fn use_shared_market() {
    let game = use_context::<Signal<Game>>();
    let remote = use_context::<Signal<Option<RemoteMarket>>>();

    let _ = use_future(move || {
        let mut game = game;
        let mut remote = remote;
        async move {
            let Some(url) = ws_url() else {
                return; // offline — nothing to do
            };
            let ws = match WebSocket::open(&url) {
                Ok(ws) => ws,
                Err(_) => return,
            };
            let (mut write, mut read) = ws.split();

            // Announce ourselves.
            let join = serde_json::to_string(&ClientMsg::Join {
                handle: "player".to_string(),
            })
            .unwrap();
            if write.send(Message::Text(join)).await.is_err() {
                return;
            }

            // Pump: wake on each server message OR a periodic timer (so trades are
            // flushed even when the server is quiet). Each iteration applies all
            // game-state changes (gossip, link refresh, fill drain, detach) in a
            // SINGLE game.write() so the persistence effect (which subscribes to
            // the whole game) wakes at most once per iteration. A non-text frame
            // is ignored (not fatal); only a socket error or close ends the loop.
            loop {
                let mut incoming_gossip: Vec<GossipEvent> = Vec::new();
                let closed = futures::select! {
                    incoming = read.next().fuse() => match incoming {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(msg) = serde_json::from_str::<ServerMsg>(&text) {
                                match msg {
                                    // Gossip feeds the engine's banter (applied below
                                    // in the coalesced write); the rest update the
                                    // separate market-view signal.
                                    ServerMsg::Gossip(events) => incoming_gossip = events,
                                    other => apply_server_msg(&mut remote, other),
                                }
                            }
                            false
                        }
                        Some(Ok(_)) => false,                 // non-text: ignore
                        Some(Err(_)) | None => true,          // errored / closed
                    },
                    _ = gloo_timers::future::TimeoutFuture::new(FLUSH_INTERVAL_MS).fuse() => false,
                };

                let (afloat, town) = {
                    let g = game.read();
                    (g.phase == Phase::River, g.state.town)
                };
                // Read the current town's shared mids from the (separate) market
                // signal before borrowing the game.
                let row = if afloat { current_row(&remote, town) } else { None };

                // Single coalesced game write: feed gossip, refresh/detach the link,
                // and drain pending fills (forwarded below, before any detach).
                let fills = {
                    let mut g = game.write();
                    if !incoming_gossip.is_empty() {
                        let feed = g.gossip.get_or_insert_with(GossipFeed::default);
                        for e in incoming_gossip {
                            feed.push(e);
                        }
                    }
                    if let Some(row) = row {
                        match g.market_link.as_mut() {
                            Some(link) => {
                                link.town = town;
                                link.quote = QuoteSnapshot { mid: row };
                            }
                            None => g.market_link = Some(MarketLink::new(town, QuoteSnapshot { mid: row })),
                        }
                    }
                    let fills = g
                        .market_link
                        .as_mut()
                        .map(|l| l.drain_fills())
                        .unwrap_or_default();
                    if !afloat {
                        g.market_link = None;
                    }
                    fills
                };

                // Forward drained trades (no game access during the awaits).
                let mut socket_dead = false;
                for fill in fills {
                    let order = ClientMsg::TradeOrder {
                        town: fill.town,
                        good: fill.good,
                        side: fill.side,
                        qty: fill.qty,
                    };
                    if let Ok(txt) = serde_json::to_string(&order) {
                        if write.send(Message::Text(txt)).await.is_err() {
                            socket_dead = true;
                            break;
                        }
                    }
                }

                if closed || socket_dead {
                    break;
                }
            }

            // Disconnected — mark it and detach. (Pending fills were forwarded each
            // loop iteration; anything recorded after the socket died can't be sent.)
            if let Some(rm) = remote.write().as_mut() {
                rm.connected = false;
            }
            let mut g = game.write();
            g.market_link = None;
            g.gossip = None;
        }
    });
}
