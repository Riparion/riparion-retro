//! Food rationing shared by the trail-style games. How well a party eats each
//! period drives both how fast the larder empties and the odds of falling ill.

use serde::{Deserialize, Serialize};

/// How well you ate this period — drives food spend and illness odds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EatLevel {
    Poorly = 1,
    Moderately = 2,
    Well = 3,
}

impl EatLevel {
    /// Food consumed: `8 + 5*E`.
    pub fn food_cost(self) -> f64 {
        8.0 + 5.0 * (self as i64 as f64)
    }
    pub fn label(self) -> &'static str {
        match self {
            EatLevel::Poorly => "Poorly",
            EatLevel::Moderately => "Moderately",
            EatLevel::Well => "Well",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn food_cost_follows_8_plus_5e() {
        assert_eq!(EatLevel::Poorly.food_cost(), 13.0);
        assert_eq!(EatLevel::Moderately.food_cost(), 18.0);
        assert_eq!(EatLevel::Well.food_cost(), 23.0);
    }
}
