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
/// 1.0 where the town has no bias for the good. `pub(crate)` so the shared
/// [`crate::market::Market`] seeds its baselines from the same formula.
pub(crate) fn mid_factor(town: usize, good: usize) -> f64 {
    let [supply, demand, _] = factors()[town][good];
    (1.0 - supply) * (1.0 + demand)
}

/// The band-1 ("floor") mid price of `good` at `town`:
/// `base_ranks[town][good] / 2 * mid_factor(town, good)` — i.e. [`generate_prices`]
/// with the 1–3 visit band pinned to 1×. Pure, RNG-free, and defined for *every*
/// town, so the dock line can compare a good's worth here against what it fetches
/// downriver, and [`crate::rumor::band_of`] can recover the rolled band as
/// `price / expected_mid`, without rolling any dice. (It's the band floor, not
/// the average — the mean band is 2× — but ratios between towns cancel the
/// constant, so the gradient hint is unaffected.)
pub(crate) fn expected_mid(town: usize, good: usize) -> f64 {
    let ranks = &scenario().river.towns[town].base_ranks;
    ranks[good] as f64 / 2.0 * mid_factor(town, good)
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

/// Roll every good's mid price for `town`: `base_ranks[town][i] / 2 *
/// mid_factor[i] * (R(3)+1)`. The 1×/2×/3× band is the per-visit volatility a
/// dockside rumor predicts. Pure RNG consumer — used both for the current town
/// and to *pre-roll* the next landing's prices into [`GameState::committed_prices`].
pub fn roll_prices(town: usize, rng: &mut GameRng) -> [f64; NUM_GOODS] {
    let ranks = &scenario().river.towns[town].base_ranks;
    let mut out = [0.0; NUM_GOODS];
    for (i, price) in out.iter_mut().enumerate() {
        *price = ranks[i] as f64 / 2.0 * mid_factor(town, i) * (rng.ri(3) + 1) as f64;
    }
    out
}

/// Set the current town's prices. If a pre-committed roll is stashed *for this
/// town* (the prices a rumor was spoken against at the previous dock), install it
/// verbatim and consume it — so the price the player faces is the one the tip
/// referred to. Otherwise roll fresh from the stream (Pittsburgh start, a
/// pre-rumor save, or — defensively — a commit whose tagged town doesn't match
/// the landing actually reached, which only the strictly-+1 river ordering should
/// ever prevent).
pub fn generate_prices(state: &mut GameState, rng: &mut GameRng) {
    state.prices = match state.committed_prices.take() {
        Some((town, committed)) if town == state.town => committed,
        _ => roll_prices(state.town, rng),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{GameState, NUM_GOODS, NUM_RIVER_TOWNS};

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
