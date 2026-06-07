//! Seedable, serializable RNG reproducing the original BASIC `FN R(X)` primitive.

use rand::{Rng as _, SeedableRng as _};
use rand_pcg::Pcg32;
use serde::{Deserialize, Serialize};

/// `FN R(X) = INT(rand * X)` from the Apple II original.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameRng(Pcg32);

impl GameRng {
    pub fn from_seed(seed: u64) -> Self {
        Self(Pcg32::seed_from_u64(seed))
    }

    /// Uniform float in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        self.0.gen::<f64>()
    }

    /// The BASIC `FN R(X)`: floor(uniform * x). For x <= 0 this is always 0,
    /// matching the original (events with sub-1 arguments can never fire).
    pub fn r(&mut self, x: f64) -> i64 {
        if x <= 0.0 {
            return 0;
        }
        (self.uniform() * x).floor() as i64
    }

    /// Convenience for integer arguments.
    pub fn ri(&mut self, x: i64) -> i64 {
        self.r(x as f64)
    }

    /// The `IF NOT(FN R(n))` idiom: true with probability 1/n.
    pub fn one_in(&mut self, n: f64) -> bool {
        self.r(n) == 0
    }
}

/// OS/browser entropy for seeding a fresh game; deterministic fallback
/// rather than panicking if the platform refuses.
pub fn random_seed() -> u64 {
    let mut bytes = [0u8; 8];
    match getrandom::getrandom(&mut bytes) {
        Ok(()) => u64::from_le_bytes(bytes),
        Err(_) => 0x7A1_9A4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_is_in_range() {
        let mut rng = GameRng::from_seed(1);
        for _ in 0..1000 {
            let v = rng.ri(3);
            assert!((0..3).contains(&v));
        }
    }

    #[test]
    fn r_of_sub_one_never_fires() {
        let mut rng = GameRng::from_seed(2);
        for _ in 0..1000 {
            assert_eq!(rng.r(0.5), 0);
        }
    }

    #[test]
    fn deterministic_for_seed() {
        let mut a = GameRng::from_seed(42);
        let mut b = GameRng::from_seed(42);
        for _ in 0..100 {
            assert_eq!(a.ri(1000), b.ri(1000));
        }
    }

    #[test]
    fn serde_round_trip_preserves_stream() {
        let mut a = GameRng::from_seed(7);
        a.ri(100); // advance
        let json = serde_json::to_string(&a).unwrap();
        let mut b: GameRng = serde_json::from_str(&json).unwrap();
        for _ in 0..100 {
            assert_eq!(a.ri(1000), b.ri(1000));
        }
    }
}
