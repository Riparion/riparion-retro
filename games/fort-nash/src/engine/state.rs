//! Core data model: the settler party's supplies, the 1779 winter calendar, and
//! screen modes.
//!
//! Field names are spelled out: `food` (provisions), `bullets` (powder & shot, a
//! *count* — $1 buys 50 rounds), `clothing` (winter clothing & blankets), `misc`
//! (supplies, doubling as the medicine chest), `oxen` (the livestock/packhorse
//! train's pulling value 200..=300 that drives mileage and is whittled down by
//! mishaps), `cash` (coin), `miles` (progress along the Wilderness Road).

use serde::{Deserialize, Serialize};

/// How well you ate this week — drives provisions spend and illness odds. The
/// ration model is shared across the trail games; see [`retro_kit::rations`].
pub use retro_kit::rations::EatLevel;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// The party name (Robertson's company, or your own).
    pub party: String,
    pub cash: f64,
    pub food: f64,
    /// Powder & shot on hand (a count of rounds, not dollars).
    pub bullets: f64,
    pub clothing: f64,
    /// Miscellaneous supplies — also your medicine chest.
    pub misc: f64,
    /// The livestock train's value/health, 200..=300 at the start; mishaps and
    /// the deepening cold gnaw at it.
    pub oxen: f64,
    pub miles: f64,
    /// Mileage at the start of the current week (for the final-leg refund).
    pub miles_at_turn_start: f64,
    /// Week counter; 0 = the day you set out (Monday, November 1, 1779).
    pub turn: u32,
    /// Self-rated rifle skill 1 (Dead-eye) .. 5 (Shaky hands); 0 if unrated.
    pub marksman: u8,
    pub ill: bool,
    pub injured: bool,
    /// One-time crossing of Powell Mountain / Wallen's Ridge (first hard pass).
    pub cleared_south_pass: bool,
    /// One-time crossing of the Cumberland Gap (second hard pass).
    pub cleared_blue_mountains: bool,
    /// One-time crossing of the frozen Cumberland River (the Christmas crossing).
    pub cleared_cumberland_river: bool,
    /// Last week's eating choice (for illness rolls and the final refund).
    pub eat_level: EatLevel,
}

impl GameState {
    pub fn new(party: String) -> Self {
        Self {
            party,
            cash: super::scenario_data::scenario().start.cash,
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
            cleared_cumberland_river: false,
            eat_level: EatLevel::Moderately,
        }
    }

    /// A station to resupply at is offered every other week (the blockhouse,
    /// Martin's Station, Mansker's Station). Week 1 has none.
    pub fn fort_available(&self) -> bool {
        self.turn.is_multiple_of(2) && self.turn > 0
    }

    pub fn date_string(&self) -> String {
        let sc = super::scenario_data::scenario();
        let idx = (self.turn as usize).min(sc.dates.len() - 1);
        format!("{} {}", sc.dates[idx], sc.calendar.year)
    }

    /// The checkpoint of the Wilderness Road you're crossing right now, by mileage
    /// — the last scenario checkpoint whose starting mile you've reached. The
    /// checkpoint mileposts line up with where the passes fire, so the displayed
    /// terrain can't drift from the geography.
    pub fn current_checkpoint(&self) -> &'static trail_kit::fortnash::Checkpoint {
        let cps = &super::scenario_data::scenario().checkpoints;
        let mut here = &cps[0];
        for cp in cps {
            if self.miles >= cp.mile {
                here = cp;
            } else {
                break;
            }
        }
        here
    }

    /// The stretch of country you're crossing, by mileage — flavor for the
    /// trail hub and a sense of progress between checkpoints.
    pub fn terrain(&self) -> &'static str {
        &self.current_checkpoint().label
    }

    /// Kebab-case cover-art slug for the current stretch, e.g. `trail-<key>`.
    pub fn terrain_key(&self) -> &'static str {
        &self.current_checkpoint().key
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

    /// Floor and clamp the perishables to zero — the clamp pass at the top of
    /// each week. Livestock is clamped too: a train can't be worth less than
    /// nothing, and an unclamped negative value would eventually drive the
    /// mileage formula `(oxen-220)/5` to absurd (even backward) travel.
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
    /// Allocating the $700 grubstake across livestock, provisions, powder &
    /// shot, winter clothing, supplies.
    Outfit,
    /// The weekly hub: hunt, stop at a station, or press on.
    Trail,
    /// Trading at a frontier station (goods cost more here).
    Fort,
    /// Choosing how well to eat this week.
    Eat,
    /// The rifle reaction game (gunfights only).
    Shoot,
    /// The shooting-gallery hunt.
    Hunt,
    /// Weaving through the breaks to shake a war party (the route-memory game).
    Flee,
    /// Picking a line over the ridges (the route-memory game).
    Climb,
    /// Finding the trace through heavy fog (the route-memory game).
    Fog,
    /// Setting a broken bone with one clean strike (the timing game).
    Splint,
    /// Measuring out a dose of medicine (the timing game).
    Dose,
    /// Holding a hand steady under strain — driving the livestock over the
    /// frozen Cumberland (the precision-trace game).
    Steady,
    /// Beating back a spreading threat — fire in camp, a soaking in freezing
    /// sleet, a guttering fire in a blizzard (the bucket-brigade game).
    Brigade,
    /// Reproducing a short ordered procedure from memory — re-seating a broken
    /// cart wheel, dressing a hurt animal's leg, working the frostbite first-aid
    /// steps in order (the order-memory game).
    Sequence,
    /// A war party ahead — choose your tactics.
    Riders,
    /// Showing the head of the pending message/prompt queue.
    Interaction,
    GameOver,
}

/// Why the journey ended — a stable tag for each death and the victory. The
/// on-screen prose lives in [`GameOverCause::message`]; [`GameOverCause::key`]
/// is the slug cover art keys on (see FORTNASH_IMAGE_KEYS.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameOverCause {
    Starved,
    Pneumonia,
    Frostbite,
    Winter,
    CantAffordDoctor,
    RiderMassacre,
    Wolves,
    IceBroke,
    Victory,
}

impl GameOverCause {
    /// Only the victory is a win.
    pub fn won(self) -> bool {
        matches!(self, GameOverCause::Victory)
    }

    /// Kebab-case cover-art slug, e.g. `game-over-<key>`.
    pub fn key(self) -> &'static str {
        match self {
            GameOverCause::Starved => "starved",
            GameOverCause::Pneumonia => "pneumonia",
            GameOverCause::Frostbite => "frostbite",
            GameOverCause::Winter => "winter",
            GameOverCause::CantAffordDoctor => "cant-tend-sick",
            GameOverCause::RiderMassacre => "raid-massacre",
            GameOverCause::Wolves => "wolves",
            GameOverCause::IceBroke => "ice-broke",
            GameOverCause::Victory => "victory",
        }
    }

    /// The cause with the given key, if any — the inverse of [`Self::key`], used
    /// to map a data-driven `Die`/`DieIfBroke` effect's cause string back to the
    /// engine's cause.
    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "starved" => GameOverCause::Starved,
            "pneumonia" => GameOverCause::Pneumonia,
            "frostbite" => GameOverCause::Frostbite,
            "winter" => GameOverCause::Winter,
            "cant-tend-sick" => GameOverCause::CantAffordDoctor,
            "raid-massacre" => GameOverCause::RiderMassacre,
            "wolves" => GameOverCause::Wolves,
            "ice-broke" => GameOverCause::IceBroke,
            "victory" => GameOverCause::Victory,
            _ => return None,
        })
    }

    /// The line shown on the game-over screen, read from the scenario's ending
    /// table (keyed by [`Self::key`]).
    pub fn message(self) -> &'static str {
        super::scenario_data::scenario()
            .ending(self.key())
            .map(|e| e.message.as_str())
            .unwrap_or("The journey ended on the trail.")
    }
}

/// How the journey ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGame {
    pub won: bool,
    pub cause: String,
    /// Stable tag for the ending; keys per-ending cover art.
    pub cause_kind: GameOverCause,
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
        assert!(!s.fort_available()); // first week: no station
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
        assert_eq!(s.date_string(), "November 1 1779");
        s.turn = 1;
        assert_eq!(s.date_string(), "November 8 1779");
        s.turn = 8;
        assert_eq!(s.date_string(), "December 27 1779");
    }
}
