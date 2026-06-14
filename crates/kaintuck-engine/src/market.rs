//! A shared, authoritative market over every river town × good.
//!
//! This is the heart of the "shared economy": instead of each `Game` rolling its
//! own private prices, many players (bots and humans) trade against ONE `Market`.
//! A buy nudges that good's price up, a sell nudges it down, and between trades
//! the price relaxes back toward its scenario baseline. The host advances time by
//! calling [`Market::tick`] with a `dt` — the engine itself stays clock-free.
//!
//! The pricing is deliberately built on the same scenario data the single-player
//! game uses: a town/good baseline is `base_ranks * mid_factor` — the *mean* of
//! the 1×/2×/3× band that [`prices::generate_prices`](crate::prices) rolls around
//! — so the shared mids sit in the same range the offline game already trades in.

use crate::market_link::{QuoteSnapshot, Side, TradeIntent};
use crate::prices::mid_factor;
use crate::scenario_data::scenario;
use crate::state::{NUM_GOODS, NUM_RIVER_TOWNS};

/// The tunable price-dynamics parameters. Defaults are the values chosen via the
/// `sim` balance harness (see `scripts/` / TODO_KAINTUCK_SERVER.md); the
/// `kaintuck-server` builds its `World` with these, and the sim can override them
/// to sweep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarketParams {
    /// How fast a disturbed price returns to baseline, per unit of `dt`. At
    /// `dt = 1.0` a price closes `1 - e^(-relax_rate)` of its gap each tick.
    pub relax_rate: f64,
    /// Fractional price move from trading `depth_units` of a good one way.
    pub impact: f64,
    /// Trade depth, in units, that moves a price ~`impact`. Larger = deeper,
    /// less twitchy market.
    pub depth_units: f64,
}

impl Default for MarketParams {
    fn default() -> Self {
        // Tuned with the `sim` harness under sustained bot load (see the sweep in
        // TODO_KAINTUCK_SERVER.md). At a typical server load these give ~2-4%
        // steady price dispersion with occasional 1.5-2x transient spikes —
        // prices visibly move and arbitrage exists, without pinning to the clamp;
        // dispersion scales smoothly with how busy the market is.
        Self {
            relax_rate: 0.03,
            impact: 0.30,
            depth_units: 60.0,
        }
    }
}

/// The live shared market: current mids, their baselines, and traded volume.
#[derive(Debug, Clone)]
pub struct Market {
    /// Current mid price per town × good.
    mids: Vec<[f64; NUM_GOODS]>,
    /// The resting price each mid relaxes toward (scenario-derived, constant).
    baselines: Vec<[f64; NUM_GOODS]>,
    /// Cumulative traded units per town × good (a metric, not a price input).
    volume: Vec<[f64; NUM_GOODS]>,
    /// Price-dynamics tuning.
    params: MarketParams,
}

impl Default for Market {
    fn default() -> Self {
        Self::new()
    }
}

impl Market {
    /// A fresh market with default tuning and every mid at its scenario baseline.
    pub fn new() -> Self {
        Self::with_params(MarketParams::default())
    }

    /// A fresh market with explicit price-dynamics tuning (for the sim sweep).
    pub fn with_params(params: MarketParams) -> Self {
        let towns = &scenario().river.towns;
        let baselines: Vec<[f64; NUM_GOODS]> = (0..NUM_RIVER_TOWNS)
            .map(|t| {
                let mut row = [0.0_f64; NUM_GOODS];
                for (g, cell) in row.iter_mut().enumerate() {
                    *cell = towns[t].base_ranks[g] as f64 * mid_factor(t, g);
                }
                row
            })
            .collect();
        Self {
            mids: baselines.clone(),
            volume: vec![[0.0; NUM_GOODS]; NUM_RIVER_TOWNS],
            baselines,
            params,
        }
    }

    /// The current mid for one good at one town.
    pub fn quote(&self, town: usize, good: usize) -> f64 {
        self.mids[town][good]
    }

    /// The resting baseline for one good at one town.
    pub fn baseline(&self, town: usize, good: usize) -> f64 {
        self.baselines[town][good]
    }

    /// Cumulative units traded for one good at one town.
    pub fn volume(&self, town: usize, good: usize) -> f64 {
        self.volume[town][good]
    }

    /// A snapshot of every mid at `town`, to hand to a `Game` via its market link.
    pub fn snapshot_for(&self, town: usize) -> QuoteSnapshot {
        QuoteSnapshot { mid: self.mids[town] }
    }

    /// Apply one trade's pressure: a buy lifts the price, a sell drops it, by an
    /// amount proportional to the quantity (monotonic in `qty`). Cheap/thin goods
    /// (low baseline) feel the same fractional move as dear ones — the move is on
    /// the price *ratio*, so absolute swings scale with the good's own price.
    pub fn apply_trade(&mut self, t: &TradeIntent) {
        if t.qty == 0 {
            return;
        }
        let signed = match t.side {
            Side::Buy => t.qty as f64,
            Side::Sell => -(t.qty as f64),
        };
        // Saturating fractional move, bounded so a single huge order can't drive a
        // price to zero or to the moon.
        let factor = (1.0 + self.params.impact * signed / self.params.depth_units).clamp(0.25, 4.0);
        self.mids[t.town][t.good] = (self.mids[t.town][t.good] * factor).max(0.01);
        self.volume[t.town][t.good] += t.qty as f64;
    }

    /// Advance time by `dt`: every mid relaxes exponentially toward its baseline.
    /// `dt` is supplied by the host (the server's wall-clock interval, or the
    /// sim's synthetic step) — the engine has no clock of its own.
    pub fn tick(&mut self, dt: f64) {
        if dt <= 0.0 {
            return;
        }
        let relax = 1.0 - (-self.params.relax_rate * dt).exp();
        for t in 0..self.mids.len() {
            for g in 0..NUM_GOODS {
                let gap = self.baselines[t][g] - self.mids[t][g];
                self.mids[t][g] += gap * relax;
            }
        }
    }

    /// Mean ratio of current mids to baselines across the whole market — 1.0 at
    /// rest, >1 when net buying has bid prices up, <1 when net selling has sunk
    /// them. A coarse "inflation under load" gauge.
    pub fn inflation(&self) -> f64 {
        let mut sum = 0.0;
        let mut n = 0.0;
        for t in 0..self.mids.len() {
            for g in 0..NUM_GOODS {
                if self.baselines[t][g] > 0.0 {
                    sum += self.mids[t][g] / self.baselines[t][g];
                    n += 1.0;
                }
            }
        }
        if n > 0.0 {
            sum / n
        } else {
            1.0
        }
    }

    /// Total units traded across the whole market.
    pub fn total_volume(&self) -> f64 {
        self.volume.iter().flat_map(|row| row.iter()).sum()
    }

    /// Mean absolute deviation of every mid from its baseline, as a fraction
    /// (0 = all prices at rest). Unlike [`inflation`](Self::inflation), upstream
    /// buy-pressure and downstream sell-pressure DON'T cancel here, so this is the
    /// real "how much is the market moving under load" gauge.
    pub fn dispersion(&self) -> f64 {
        let mut sum = 0.0;
        let mut n = 0.0;
        for t in 0..self.mids.len() {
            for g in 0..NUM_GOODS {
                if self.baselines[t][g] > 0.0 {
                    sum += (self.mids[t][g] / self.baselines[t][g] - 1.0).abs();
                    n += 1.0;
                }
            }
        }
        if n > 0.0 {
            sum / n
        } else {
            0.0
        }
    }

    /// The single most-displaced mid, as a fraction off baseline.
    pub fn max_dispersion(&self) -> f64 {
        let mut m = 0.0_f64;
        for t in 0..self.mids.len() {
            for g in 0..NUM_GOODS {
                if self.baselines[t][g] > 0.0 {
                    m = m.max((self.mids[t][g] / self.baselines[t][g] - 1.0).abs());
                }
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_link::{Side, TradeIntent};

    fn trade(town: usize, good: usize, side: Side, qty: i64) -> TradeIntent {
        TradeIntent {
            town,
            good,
            side,
            qty,
            unit_price: 0.0,
        }
    }

    #[test]
    fn a_buy_raises_and_a_sell_lowers_the_price() {
        let mut m = Market::new();
        let before = m.quote(2, 1);
        m.apply_trade(&trade(2, 1, Side::Buy, 50));
        assert!(m.quote(2, 1) > before, "buying should raise the price");

        let mut m = Market::new();
        let before = m.quote(2, 1);
        m.apply_trade(&trade(2, 1, Side::Sell, 50));
        assert!(m.quote(2, 1) < before, "selling should lower the price");
    }

    #[test]
    fn bigger_orders_move_the_price_more() {
        let mut small = Market::new();
        let mut big = Market::new();
        let base = small.quote(0, 0);
        small.apply_trade(&trade(0, 0, Side::Buy, 10));
        big.apply_trade(&trade(0, 0, Side::Buy, 100));
        assert!(big.quote(0, 0) - base > small.quote(0, 0) - base);
    }

    #[test]
    fn tick_relaxes_a_disturbed_price_back_toward_baseline() {
        let mut m = Market::new();
        let baseline = m.baseline(3, 2);
        m.apply_trade(&trade(3, 2, Side::Buy, 80));
        let disturbed = m.quote(3, 2);
        assert!(disturbed > baseline);
        // Many ticks should pull it most of the way home.
        for _ in 0..100 {
            m.tick(1.0);
        }
        let settled = m.quote(3, 2);
        assert!((settled - baseline).abs() < (disturbed - baseline).abs() * 0.05);
    }

    #[test]
    fn a_zero_dt_tick_changes_nothing() {
        let mut m = Market::new();
        m.apply_trade(&trade(1, 1, Side::Buy, 30));
        let before = m.quote(1, 1);
        m.tick(0.0);
        assert_eq!(m.quote(1, 1), before);
    }
}
