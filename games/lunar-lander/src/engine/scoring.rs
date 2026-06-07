//! Landing classification (faithful to the originals' thresholds) and the
//! score scheme — the 1978 listings kept no scores, so ours rewards fuel
//! economy plus a softness bonus, per mission.

use serde::{Deserialize, Serialize};

use super::state::{EndGame, Landing, MissionKind};

/// lunar.bas lines 274–320.
pub fn classify_lunar(mph: f64) -> Landing {
    if mph <= 1.2 {
        Landing::Perfect
    } else if mph <= 10.0 {
        Landing::Good
    } else if mph <= 60.0 {
        Landing::Damaged
    } else {
        Landing::Crash
    }
}

/// rocket.bas lines 790–830.
pub fn classify_rocket(landing_v: f64) -> Landing {
    #[allow(clippy::float_cmp)] // faithful to BASIC's IF V1<>0
    if landing_v == 0.0 {
        Landing::Perfect
    } else if landing_v.abs() < 2.0 {
        Landing::Safe
    } else {
        Landing::Crash
    }
}

fn softness_bonus(quality: Landing) -> i64 {
    match quality {
        Landing::Perfect => 2_000,
        Landing::Good | Landing::Safe => 500,
        Landing::Damaged | Landing::Crash => 0,
    }
}

/// Successful landings only; crashes and strandings score zero.
pub fn score(mission: MissionKind, quality: Landing, fuel_left: f64) -> i64 {
    if !quality.landed_ok() {
        return 0;
    }
    let fuel_points = match mission {
        MissionKind::Lunar => (fuel_left / 10.0).floor() as i64,
        MissionKind::Rocket => (fuel_left * 30.0).floor() as i64,
    };
    fuel_points + softness_bonus(quality)
}

pub fn lunar_end_game(mph: f64, elapsed: f64, fuel_left: f64) -> EndGame {
    let quality = classify_lunar(mph);
    let (headline, detail) = match quality {
        Landing::Perfect => ("PERFECT LANDING!".to_string(), "LUCKY GEEZER.".to_string()),
        Landing::Good => (
            "GOOD LANDING (COULD BE BETTER)".to_string(),
            String::new(),
        ),
        Landing::Damaged => (
            "CRAFT DAMAGE...".to_string(),
            "You're stranded here until a rescue party arrives. \
             Hope you have enough oxygen!"
                .to_string(),
        ),
        _ => (
            "SORRY, THERE WERE NO SURVIVORS. YOU BLEW IT!".to_string(),
            format!(
                "In fact, you blasted a new lunar crater {:.1} feet deep!",
                mph * 0.227
            ),
        ),
    };
    EndGame {
        mission: MissionKind::Lunar,
        quality,
        headline,
        detail,
        impact: format!("{mph:.1} MPH"),
        elapsed,
        fuel_left,
        score: score(MissionKind::Lunar, quality, fuel_left),
        recorded: false,
    }
}

pub fn rocket_end_game(landing_v: f64, touchdown_at: f64, fuel_left: f64) -> EndGame {
    let quality = classify_rocket(landing_v);
    let (headline, detail) = match quality {
        Landing::Perfect => (
            "CONGRATULATIONS! A PERFECT LANDING!!".to_string(),
            "Your license will be renewed.......later.".to_string(),
        ),
        Landing::Safe => ("A SAFE LANDING — WELL DONE.".to_string(), String::new()),
        _ => (
            "SORRY, BUT YOU BLEW IT!!!!".to_string(),
            "Appropriate condolences will be sent to your next of kin.".to_string(),
        ),
    };
    EndGame {
        mission: MissionKind::Rocket,
        quality,
        headline,
        detail,
        impact: format!("{:.1} FT/S", landing_v.abs()),
        elapsed: touchdown_at,
        fuel_left,
        score: score(MissionKind::Rocket, quality, fuel_left),
        recorded: false,
    }
}

/// One row of the local hall of fame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighScore {
    pub mission: MissionKind,
    pub score: i64,
    pub quality: String,
    pub impact: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lunar_boundaries() {
        assert_eq!(classify_lunar(1.2), Landing::Perfect);
        assert_eq!(classify_lunar(1.21), Landing::Good);
        assert_eq!(classify_lunar(10.0), Landing::Good);
        assert_eq!(classify_lunar(10.01), Landing::Damaged);
        assert_eq!(classify_lunar(60.0), Landing::Damaged);
        assert_eq!(classify_lunar(60.01), Landing::Crash);
    }

    #[test]
    fn rocket_boundaries() {
        assert_eq!(classify_rocket(0.0), Landing::Perfect);
        assert_eq!(classify_rocket(1.9), Landing::Safe);
        assert_eq!(classify_rocket(-1.9), Landing::Safe);
        assert_eq!(classify_rocket(2.0), Landing::Crash);
    }

    #[test]
    fn crater_depth_in_crash_text() {
        let end = lunar_end_game(100.0, 50.0, 0.0);
        assert!(end.detail.contains("22.7 feet deep"));
        assert_eq!(end.score, 0);
    }

    #[test]
    fn scores_reward_fuel_and_softness() {
        assert_eq!(score(MissionKind::Lunar, Landing::Perfect, 8_000.0), 2_800);
        assert_eq!(score(MissionKind::Lunar, Landing::Good, 8_000.0), 1_300);
        assert_eq!(score(MissionKind::Lunar, Landing::Damaged, 8_000.0), 0);
        assert_eq!(score(MissionKind::Rocket, Landing::Perfect, 50.0), 3_500);
        assert_eq!(score(MissionKind::Rocket, Landing::Safe, 50.0), 2_000);
        assert_eq!(score(MissionKind::Rocket, Landing::Crash, 50.0), 0);
    }
}
