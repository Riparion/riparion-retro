//! Price generation, exactly as the BASIC original.

use super::rng::GameRng;
use super::state::GameState;

/// Regenerate all four prices for the current port:
/// `CP(I) = BP%(LO,I) / 2 * (R(3)+1) * 10^(4-I)`.
pub fn generate_prices(state: &mut GameState, rng: &mut GameRng) {
    debug_assert!(state.port >= 1);
    let ranks = state.base_ranks[state.port - 1];
    for (i, price) in state.prices.iter_mut().enumerate() {
        let magnitude = 10f64.powi(3 - i as i32); // Opium x1000 .. General x1
        *price = ranks[i] as f64 / 2.0 * (rng.ri(3) + 1) as f64 * magnitude;
    }
}

/// Yearly inflation: every base rank rises by `R(2)` (0 or 1).
pub fn inflate_base_ranks(state: &mut GameState, rng: &mut GameRng) {
    for port in state.base_ranks.iter_mut() {
        for rank in port.iter_mut() {
            *rank += rng.ri(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{StartChoice, HONG_KONG};

    #[test]
    fn prices_are_within_original_bounds() {
        let mut state = GameState::new("T".into(), StartChoice::Cash);
        let mut rng = GameRng::from_seed(9);
        for port in 1..=7 {
            state.port = port;
            for _ in 0..200 {
                generate_prices(&mut state, &mut rng);
                let ranks = state.base_ranks[port - 1];
                for (i, &rank) in ranks.iter().enumerate() {
                    let base = rank as f64 / 2.0 * 10f64.powi(3 - i as i32);
                    let p = state.prices[i];
                    // multiplier is 1, 2 or 3
                    assert!(
                        (p - base).abs() < 1e-9
                            || (p - 2.0 * base).abs() < 1e-9
                            || (p - 3.0 * base).abs() < 1e-9,
                        "port {port} item {i}: price {p} not in {{1,2,3}}x{base}"
                    );
                }
            }
        }
    }

    #[test]
    fn hong_kong_opium_price_examples() {
        // HK opium rank 11 => 5.5 * {1,2,3} * 1000 = 5500/11000/16500
        let mut state = GameState::new("T".into(), StartChoice::Cash);
        state.port = HONG_KONG;
        let mut rng = GameRng::from_seed(1);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            generate_prices(&mut state, &mut rng);
            seen.insert(state.prices[0] as i64);
        }
        assert_eq!(
            seen,
            [5500i64, 11000, 16500].into_iter().collect::<std::collections::HashSet<_>>()
        );
    }

    #[test]
    fn inflation_only_raises_ranks() {
        let mut state = GameState::new("T".into(), StartChoice::Cash);
        let before = state.base_ranks;
        let mut rng = GameRng::from_seed(3);
        inflate_base_ranks(&mut state, &mut rng);
        for (after_port, before_port) in state.base_ranks.iter().zip(before.iter()) {
            for (after, before) in after_port.iter().zip(before_port.iter()) {
                assert!((0..=1).contains(&(after - before)));
            }
        }
    }
}
