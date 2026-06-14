//! Market price generation, in the taipan/BASIC idiom: each good's quote at a
//! landing is half its base rank times a 1–3 multiplier, so the same town swings
//! goods 1×/2×/3× of the rank-mean from visit to visit.
//!
//! On top of that distance gradient sits a *local* economic bias (`Town::market`
//! in the scenario): a `supply` discount on goods a town produces, a `demand`
//! premium on goods it lacks. They scale the rank-mean before the 1–3 band, so
//! the quote stored in `state.prices` is the local **mid**. The buy/sell
//! [`spread`] is applied around that mid at the trade boundary in `mod.rs`.

use std::sync::OnceLock;

use super::rng::GameRng;
use super::scenario_data::scenario;
use super::state::{GameState, GOOD_NAMES, NUM_GOODS};

/// Per-town, per-good `[supply, demand, spread]`, resolved once from the
/// scenario's sparse `market` lists (good name → index). Towns and goods with no
/// bias stay all-zero, i.e. the plain gradient and the scenario base spread.
fn factors() -> &'static [[[f64; 3]; NUM_GOODS]] {
    static CACHE: OnceLock<Vec<[[f64; 3]; NUM_GOODS]>> = OnceLock::new();
    CACHE.get_or_init(|| {
        scenario()
            .river
            .towns
            .iter()
            .map(|town| {
                let mut f = [[0.0_f64; 3]; NUM_GOODS];
                for bias in &town.market {
                    let i = GOOD_NAMES
                        .iter()
                        .position(|n| *n == bias.good)
                        .unwrap_or_else(|| panic!("market bias for unknown good {:?}", bias.good));
                    f[i] = [bias.supply, bias.demand, bias.spread];
                }
                f
            })
            .collect()
    })
}

/// The local mid-price multiplier on `good` at `town`: `(1 - supply)(1 + demand)`.
/// 1.0 where the town has no bias for the good.
fn mid_factor(town: usize, good: usize) -> f64 {
    let [supply, demand, _] = factors()[town][good];
    (1.0 - supply) * (1.0 + demand)
}

/// Whether `town` locally produces `good` — it carries a supply (production)
/// discount on it, the same bias that drives the dock's "cheap here; they make it
/// by the boatload" line. Used to flag the good on the trade screen.
pub fn is_locally_produced(town: usize, good: usize) -> bool {
    factors()[town][good][0] > 0.0
}

/// The combined bid/ask spread on `good` at `town`: the scenario base spread plus
/// the town's local widening, clamped below 1.0 so the bid can't go non-positive.
pub fn spread(town: usize, good: usize) -> f64 {
    (scenario().start.base_spread + factors()[town][good][2]).clamp(0.0, 0.95)
}

/// Regenerate every good's mid price for the current town:
/// `price[i] = base_ranks[town][i] / 2 * mid_factor[i] * (R(3)+1)`.
pub fn generate_prices(state: &mut GameState, rng: &mut GameRng) {
    let town = state.town;
    let ranks = &scenario().river.towns[town].base_ranks;
    for (i, price) in state.prices.iter_mut().enumerate() {
        *price = ranks[i] as f64 / 2.0 * mid_factor(town, i) * (rng.ri(3) + 1) as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{GameState, NUM_GOODS, NUM_RIVER_TOWNS};

    #[test]
    fn prices_stay_within_the_one_to_three_band() {
        let mut state = GameState::new("T".into());
        let mut rng = GameRng::from_seed(9);
        for town in 0..NUM_RIVER_TOWNS {
            state.town = town;
            for _ in 0..200 {
                generate_prices(&mut state, &mut rng);
                for i in 0..NUM_GOODS {
                    // The mid still lands on 1×/2×/3× the rank-mean — but the
                    // rank-mean is now scaled by the town's local supply/demand
                    // factor, so fold that in before checking the band.
                    let base =
                        scenario().river.towns[town].base_ranks[i] as f64 / 2.0 * mid_factor(town, i);
                    let p = state.prices[i];
                    assert!(
                        (p - base).abs() < 1e-9
                            || (p - 2.0 * base).abs() < 1e-9
                            || (p - 3.0 * base).abs() < 1e-9,
                        "town {town} good {i}: {p} not in {{1,2,3}}x{base}"
                    );
                }
            }
        }
    }

    #[test]
    fn all_prices_are_positive() {
        let mut state = GameState::new("T".into());
        let mut rng = GameRng::from_seed(3);
        for town in 0..NUM_RIVER_TOWNS {
            state.town = town;
            generate_prices(&mut state, &mut rng);
            assert!(state.prices.iter().all(|&p| p > 0.0));
        }
    }
}
