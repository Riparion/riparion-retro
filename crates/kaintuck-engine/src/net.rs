//! The client ↔ server wire protocol for the shared-economy game server.
//!
//! These are pure `serde` message types with no transport attached — both the
//! wasm UI client and the native (axum/WebSocket) server depend on this module,
//! so they can't drift out of sync. The transport (WebSocket framing, JSON
//! encoding) lives in each end.
//!
//! The model is *independent journeys, shared market*: a human runs their whole
//! journey locally in the UI; only their port-market trades cross the wire. So
//! the protocol is small — join, submit a trade order, receive market updates
//! and your fills.

use serde::{Deserialize, Serialize};

use crate::market_link::Side;
use crate::state::{NUM_GOODS, NUM_RIVER_TOWNS};

/// Every river town's current mids — the whole shared market, as the server
/// holds it. Sent on join and whenever a client asks to resync.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSnapshot {
    /// `mids[town][good]` — the authoritative mid price.
    pub mids: Vec<[f64; NUM_GOODS]>,
}

impl MarketSnapshot {
    /// The number of towns this snapshot covers (should be [`NUM_RIVER_TOWNS`]).
    pub fn towns(&self) -> usize {
        self.mids.len()
    }

    /// A snapshot of a market-at-rest's baselines, for tests/clients with no
    /// connection yet. (Real snapshots come from the server's live market.)
    pub fn at_rest() -> Self {
        let m = crate::market::Market::new();
        let mids = (0..NUM_RIVER_TOWNS).map(|t| m.snapshot_for(t).mid).collect();
        Self { mids }
    }
}

/// Client → server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMsg {
    /// Join the shared world under a display handle.
    Join { handle: String },
    /// Place a market order at `town` for `good`: buy/sell `qty` units. The
    /// server prices it against the live shared market and replies with a fill.
    TradeOrder {
        town: usize,
        good: usize,
        side: Side,
        qty: i64,
    },
    /// Ask for a fresh full snapshot (e.g. after a reconnect).
    Resync,
    /// Leave the world.
    Leave,
}

/// Server → client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMsg {
    /// Acknowledges a join: the player's id and the current market.
    Welcome {
        participant_id: u64,
        snapshot: MarketSnapshot,
    },
    /// A full market snapshot (reply to [`ClientMsg::Resync`]).
    Snapshot(MarketSnapshot),
    /// One town's mids changed (someone traded, or relaxation moved them). Sent
    /// to everyone so each client's local quotes stay current.
    MarketDelta { town: usize, mids: [f64; NUM_GOODS] },
    /// The result of this client's own [`ClientMsg::TradeOrder`].
    TradeFill {
        town: usize,
        good: usize,
        side: Side,
        qty: i64,
        /// The shared-market **mid** the order executed against (the reference
        /// price), NOT the player's net unit cost — the client's own engine
        /// applies the bid/ask spread and reputation to get the actual fill, so
        /// the server deliberately doesn't claim a net price it didn't compute.
        mid: f64,
    },
    /// Attributed activity by *other* traders (bots) in the shared world —
    /// batched per server tick. The client feeds these to the engine, whose crew
    /// voices them as banter. See [`crate::gossip`].
    Gossip(Vec<crate::gossip::GossipEvent>),
    /// A recoverable error (bad order, unknown town/good, etc.).
    Error { message: String },
}
