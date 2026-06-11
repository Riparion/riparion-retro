//! Market price generation, in the taipan/BASIC idiom: each good's quote at a
//! landing is half its base rank times a 1–3 multiplier, so the same town swings
//! goods 1×/2×/3× of the rank-mean from visit to visit.

use super::rng::GameRng;
use super::state::GameState;

/// Regenerate every good's price for the current town:
/// `price[i] = base_ranks[town][i] / 2 * (R(3)+1)`.
pub fn generate_prices(state: &mut GameState, rng: &mut GameRng) {
    let ranks = state.base_ranks[state.town];
    for (i, price) in state.prices.iter_mut().enumerate() {
        *price = ranks[i] as f64 / 2.0 * (rng.ri(3) + 1) as f64;
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
                    let base = state.base_ranks[town][i] as f64 / 2.0;
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
