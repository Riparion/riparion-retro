//! The opt-in seam that lets a [`Game`](crate::Game) trade against a *shared*
//! market instead of its own privately-rolled prices.
//!
//! By default a `Game` carries no link (`Game::market_link == None`) and behaves
//! exactly as the single-player game always has: prices come from
//! [`prices::generate_prices`](crate::prices::generate_prices) into
//! `state.prices`, and trades touch nothing outside the `Game`. This module is
//! pure data; the field on `Game` is `#[serde(skip)]`, so attaching a link never
//! changes how a save serializes.
//!
//! When a host (the economy simulator or the multiplayer server, via the UI)
//! attaches a [`MarketLink`], two things change at the trade boundary:
//!
//! 1. `buy_price`/`sell_price` read the *mid* from the link's [`QuoteSnapshot`]
//!    rather than from `state.prices` — the same spread + reputation math is then
//!    applied on top, so the pricing pipeline is otherwise unchanged.
//! 2. each `buy`/`sell` records a [`TradeIntent`] on the link, which the host
//!    drains and applies to the authoritative shared market (moving the price for
//!    everyone) after the engine step.
//!
//! Crucially, the engine still *rolls* `generate_prices` on arrival even when a
//! link is attached — the link merely *shadows* the rolled mid at the trade
//! boundary. That keeps the RNG stream (and the golden trace) byte-identical.

use serde::{Deserialize, Serialize};

use crate::state::NUM_GOODS;

/// Which way a trade went, recorded on a [`TradeIntent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

/// The shared market's current mid quote for every good at one town. The host
/// pushes this in when the player is at a market; the engine reads it in
/// `buy_price`/`sell_price` in place of `state.prices`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuoteSnapshot {
    pub mid: [f64; NUM_GOODS],
}

impl QuoteSnapshot {
    /// A snapshot built straight from a `state.prices`-shaped slice.
    pub fn from_prices(prices: &[f64; NUM_GOODS]) -> Self {
        Self { mid: *prices }
    }
}

/// One trade the player made while linked, for the host to apply to the shared
/// market. `unit_price` is the actual fill price the engine charged (spread and,
/// for sells, reputation already folded in).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TradeIntent {
    pub town: usize,
    pub good: usize,
    pub side: Side,
    pub qty: i64,
    pub unit_price: f64,
}

/// A live connection from a `Game` to a shared market: the latest quote for the
/// town the player is in, plus the trades made since the host last drained them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketLink {
    /// The town this quote is for (matches `state.town` while at a market).
    pub town: usize,
    /// The shared market's current mids for this town.
    pub quote: QuoteSnapshot,
    /// Trades made since the host last drained; the host applies and clears these.
    pub fills: Vec<TradeIntent>,
}

impl MarketLink {
    /// A fresh link for `town` seeded with the given mids and no pending fills.
    pub fn new(town: usize, quote: QuoteSnapshot) -> Self {
        Self {
            town,
            quote,
            fills: Vec::new(),
        }
    }

    /// Take the recorded trades, leaving the link ready to record the next step.
    pub fn drain_fills(&mut self) -> Vec<TradeIntent> {
        std::mem::take(&mut self.fills)
    }
}
