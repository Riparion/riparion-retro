//! Core data model: the wagon party's supplies, the calendar, and screen modes.
//!
//! Field names follow the 1975 MECC BASIC where it helps cross-checking the
//! spec, but spelled out: `food` (F), `bullets` (B, a *count* — $1 buys 50),
//! `clothing` (C), `misc` (M1, doubles as medicine), `oxen` (A, the team's
//! pulling value 200..=300 that also drives mileage and is whittled down by
//! mishaps), `cash` (T), `miles` (M).

use serde::{Deserialize, Serialize};

/// Independence, Missouri → Oregon City: the whole trail.
pub const TRAIL_MILES: f64 = 2040.0;
/// $1 buys a belt of 50 bullets.
pub const BULLETS_PER_DOLLAR: f64 = 50.0;
/// Bullets spent per shot fired in the hunting gallery.
pub const BULLETS_PER_SHOT: f64 = 5.0;
/// You start with $700 after the wagon, family of five, spring of 1847.
pub const STARTING_CASH: f64 = 700.0;
/// Mile mark past which the mountains (South Pass, blizzards) begin.
pub const MOUNTAINS_AT: f64 = 950.0;
/// Mile mark of the Blue Mountains / final approach.
pub const BLUE_MOUNTAINS_AT: f64 = 1700.0;
/// 20 fortnights out and winter buries you.
pub const MAX_TURNS: u32 = 20;

/// Date at the start of each fortnight, indexed by turn (turn 0 = the day you
/// roll out of Independence). Turn `MAX_TURNS` is past the end — winter death.
pub const DATES: [&str; MAX_TURNS as usize] = [
    "March 29",
    "April 12",
    "April 26",
    "May 10",
    "May 24",
    "June 7",
    "June 21",
    "July 5",
    "July 19",
    "August 2",
    "August 16",
    "August 31",
    "September 13",
    "September 27",
    "October 11",
    "October 25",
    "November 8",
    "November 22",
    "December 6",
    "December 20",
];

/// How well you ate this fortnight — drives food spend and illness odds.
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// The family name (for the trailside marker if it comes to that).
    pub party: String,
    pub cash: f64,
    pub food: f64,
    /// Bullets on hand (a count, not dollars).
    pub bullets: f64,
    pub clothing: f64,
    /// Miscellaneous supplies — also your medicine chest.
    pub misc: f64,
    /// The oxen team's value/health, 200..=300 at the start; mishaps gnaw at it.
    pub oxen: f64,
    pub miles: f64,
    /// Mileage at the start of the current fortnight (for the final-leg refund).
    pub miles_at_turn_start: f64,
    /// Fortnight counter; 0 = the day you set out (March 29, 1847).
    pub turn: u32,
    /// Self-rated marksmanship 1 (Ace) .. 5 (Shaky knees); 0 if unrated.
    pub marksman: u8,
    pub ill: bool,
    pub injured: bool,
    pub cleared_south_pass: bool,
    pub cleared_blue_mountains: bool,
    /// Last fortnight's eating choice (for illness rolls and the final refund).
    pub eat_level: EatLevel,
}

impl GameState {
    pub fn new(party: String) -> Self {
        Self {
            party,
            cash: STARTING_CASH,
            food: 0.0,
            bullets: 0.0,
            clothing: 0.0,
            misc: 0.0,
            oxen: 0.0,
            miles: 0.0,
            miles_at_turn_start: 0.0,
            turn: 0,
            marksman: 0,
            ill: false,
            injured: false,
            cleared_south_pass: false,
            cleared_blue_mountains: false,
            eat_level: EatLevel::Moderately,
        }
    }

    /// A fort is offered every other fortnight (turn 1 has none, matching the
    /// original's `X1` toggle that starts at "no fort").
    pub fn fort_available(&self) -> bool {
        self.turn.is_multiple_of(2) && self.turn > 0
    }

    pub fn date_string(&self) -> String {
        let idx = (self.turn as usize).min(DATES.len() - 1);
        format!("{} 1847", DATES[idx])
    }

    /// The stretch of country you're crossing, by mileage — flavor for the
    /// trail hub and a sense of progress between landmarks.
    pub fn terrain(&self) -> &'static str {
        match self.miles {
            m if m < 320.0 => "The Kansas plains",
            m if m < 550.0 => "The Platte River valley",
            m if m < 640.0 => "Fort Laramie country",
            m if m < MOUNTAINS_AT => "The high plains",
            m if m < BLUE_MOUNTAINS_AT => "The Rocky Mountains — South Pass",
            m if m < TRAIL_MILES => "The Blue Mountains",
            _ => "The Willamette Valley",
        }
    }

    /// Healthy / Ill / Injured for the status line.
    pub fn health_label(&self) -> &'static str {
        if self.injured {
            "Injured"
        } else if self.ill {
            "Ill"
        } else {
            "Healthy"
        }
    }

    /// Floor and clamp the perishables to zero — the original `INT`/clamp pass
    /// at the top of each fortnight. Oxen is clamped too: a team can't be worth
    /// less than nothing, and an unclamped negative value would eventually drive
    /// the mileage formula `(oxen-220)/5` to absurd (even backward) travel.
    pub fn clamp_supplies(&mut self) {
        for v in [
            &mut self.food,
            &mut self.bullets,
            &mut self.clothing,
            &mut self.misc,
            &mut self.cash,
            &mut self.oxen,
        ] {
            *v = v.floor().max(0.0);
        }
        self.miles = self.miles.floor();
    }
}

/// Which screen is showing. Saved with the game so a refresh resumes in place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    Splash,
    NewGame,
    /// Allocating the $700 grubstake across oxen, food, ammo, clothing, supplies.
    Outfit,
    /// The fortnight hub: hunt, stop at a fort, or press on.
    Trail,
    /// Shopping at a fort (goods cost more here).
    Fort,
    /// Choosing how well to eat this fortnight.
    Eat,
    /// The marksmanship reaction game (gunfights only).
    Shoot,
    /// The shooting-gallery hunt.
    Hunt,
    /// Riders ahead — choose your tactics.
    Riders,
    /// Showing the head of the pending message/prompt queue.
    Interaction,
    GameOver,
}

/// How the journey ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGame {
    pub won: bool,
    pub cause: String,
    /// Arrival date string, when you actually made it.
    pub arrival: Option<String>,
    pub miles: i64,
    pub days: i64,
    pub food: i64,
    pub bullets: i64,
    pub clothing: i64,
    pub misc: i64,
    pub cash: i64,
    pub score: i64,
    pub rank: String,
    pub recorded: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eat_costs_match_basic() {
        assert_eq!(EatLevel::Poorly.food_cost(), 13.0);
        assert_eq!(EatLevel::Moderately.food_cost(), 18.0);
        assert_eq!(EatLevel::Well.food_cost(), 23.0);
    }

    #[test]
    fn fort_offered_every_other_turn() {
        let mut s = GameState::new("T".into());
        s.turn = 0;
        assert!(!s.fort_available()); // setting out
        s.turn = 1;
        assert!(!s.fort_available()); // first fortnight: no fort
        s.turn = 2;
        assert!(s.fort_available());
        s.turn = 3;
        assert!(!s.fort_available());
        s.turn = 4;
        assert!(s.fort_available());
    }

    #[test]
    fn dates_advance() {
        let mut s = GameState::new("T".into());
        assert_eq!(s.date_string(), "March 29 1847");
        s.turn = 1;
        assert_eq!(s.date_string(), "April 12 1847");
        s.turn = 19;
        assert_eq!(s.date_string(), "December 20 1847");
    }
}
