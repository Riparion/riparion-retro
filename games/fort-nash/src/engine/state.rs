//! Core data model: the settler party's supplies, the 1779 winter calendar, and
//! screen modes.
//!
//! Field names are spelled out: `food` (provisions), `bullets` (powder & shot, a
//! *count* — $1 buys 50 rounds), `clothing` (winter clothing & blankets), `misc`
//! (supplies, doubling as the medicine chest), `oxen` (the livestock/packhorse
//! train's pulling value 200..=300 that drives mileage and is whittled down by
//! mishaps), `cash` (coin), `miles` (progress along the Wilderness Road).

use serde::{Deserialize, Serialize};

/// Fort Patrick Henry (Kingsport) → Fort Nashborough (French Lick): the whole
/// Wilderness Road, on the engine's stylized mileage scale.
pub const TRAIL_MILES: f64 = 1600.0;
/// $1 buys a horn of 50 rounds.
pub const BULLETS_PER_DOLLAR: f64 = 50.0;
/// Rounds spent per shot fired in the hunting gallery.
pub const BULLETS_PER_SHOT: f64 = 5.0;
/// You start with $700 in coin and trade goods, Kingsport, autumn of 1779.
pub const STARTING_CASH: f64 = 700.0;
/// Mile mark past which the ridges (Powell Mountain, Wallen's Ridge) begin.
pub const MOUNTAINS_AT: f64 = 360.0;
/// Mile mark of the Cumberland Gap — the passage through the Appalachian spine.
pub const CUMBERLAND_GAP_AT: f64 = 760.0;
/// Mile mark of the frozen Cumberland River crossing (the Christmas crossing).
pub const CUMBERLAND_RIVER_AT: f64 = 1400.0;
/// 9 weeks out of Kingsport and the deep winter buries you on the trail.
pub const MAX_TURNS: u32 = 9;

/// Date at the start of each week, indexed by turn (turn 0 = the Monday you
/// march out of Fort Patrick Henry, November 1, 1779). Turn `MAX_TURNS` is past
/// the end — caught by winter.
pub const DATES: [&str; MAX_TURNS as usize] = [
    "November 1",
    "November 8",
    "November 15",
    "November 22",
    "November 29",
    "December 6",
    "December 13",
    "December 20",
    "December 27",
];

/// How well you ate this week — drives provisions spend and illness odds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EatLevel {
    Poorly = 1,
    Moderately = 2,
    Well = 3,
}

impl EatLevel {
    /// Provisions consumed: `8 + 5*E`.
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
        let idx = (self.turn as usize).min(DATES.len() - 1);
        format!("{} 1779", DATES[idx])
    }

    /// The checkpoint of the Wilderness Road you're crossing right now, by mileage.
    pub fn terrain_kind(&self) -> Terrain {
        match self.miles {
            m if m < 180.0 => Terrain::FortPatrickHenry,
            m if m < MOUNTAINS_AT => Terrain::MoccasinGap,
            m if m < 560.0 => Terrain::PowellValley,
            m if m < CUMBERLAND_GAP_AT => Terrain::MartinsStation,
            m if m < 1000.0 => Terrain::CumberlandGap,
            m if m < 1200.0 => Terrain::CrabOrchard,
            m if m < CUMBERLAND_RIVER_AT => Terrain::KentuckyBarrens,
            m if m < TRAIL_MILES => Terrain::CumberlandRiver,
            _ => Terrain::FortNashborough,
        }
    }

    /// The stretch of country you're crossing, by mileage — flavor for the
    /// trail hub and a sense of progress between checkpoints.
    pub fn terrain(&self) -> &'static str {
        self.terrain_kind().label()
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

/// The checkpoint of the Wilderness Road you're crossing, by mileage. The
/// on-trail prose lives in [`Terrain::label`]; [`Terrain::key`] is the slug
/// location-specific trail cover art keys on, e.g. `trail-<key>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terrain {
    FortPatrickHenry,
    MoccasinGap,
    PowellValley,
    MartinsStation,
    CumberlandGap,
    CrabOrchard,
    KentuckyBarrens,
    CumberlandRiver,
    FortNashborough,
}

impl Terrain {
    /// The "where you are" line shown on the trail hub.
    pub fn label(self) -> &'static str {
        match self {
            Terrain::FortPatrickHenry => "Fort Patrick Henry",
            Terrain::MoccasinGap => "Moccasin Gap",
            Terrain::PowellValley => "Powell Valley",
            Terrain::MartinsStation => "Martin's Station",
            Terrain::CumberlandGap => "The Cumberland Gap",
            Terrain::CrabOrchard => "Crab Orchard",
            Terrain::KentuckyBarrens => "The Kentucky barrens",
            Terrain::CumberlandRiver => "The Cumberland River",
            Terrain::FortNashborough => "Fort Nashborough",
        }
    }

    /// Kebab-case cover-art slug, e.g. `trail-<key>`.
    pub fn key(self) -> &'static str {
        match self {
            Terrain::FortPatrickHenry => "fort-patrick-henry",
            Terrain::MoccasinGap => "moccasin-gap",
            Terrain::PowellValley => "powell-valley",
            Terrain::MartinsStation => "martins-station",
            Terrain::CumberlandGap => "cumberland-gap",
            Terrain::CrabOrchard => "crab-orchard",
            Terrain::KentuckyBarrens => "kentucky-barrens",
            Terrain::CumberlandRiver => "cumberland-river",
            Terrain::FortNashborough => "fort-nashborough",
        }
    }
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

    /// The line shown on the game-over screen.
    pub fn message(self) -> &'static str {
        match self {
            GameOverCause::Starved => {
                "Your provisions ran out and the party starved on the trail."
            }
            GameOverCause::Pneumonia => {
                "Your medicine ran out and the sickness turned to pneumonia in the cold."
            }
            GameOverCause::Frostbite => {
                "Frostbite took hold with no medicine left to treat it — the party could go no farther."
            }
            GameOverCause::Winter => {
                "You've been on the trail too long. The deep winter closes the road and buries the party in the first hard blizzard."
            }
            GameOverCause::CantAffordDoctor => {
                "With nothing left to trade for care, the sickness took the party."
            }
            GameOverCause::RiderMassacre => {
                "Your powder ran out and the war party overran the camp."
            }
            GameOverCause::Wolves => "The wolves overpowered you. You died of your injuries.",
            GameOverCause::IceBroke => {
                "The ice gave way under the livestock and the party went into the freezing Cumberland."
            }
            GameOverCause::Victory => {
                "You reached the French Lick and raised Fort Nashborough on the Cumberland — December 1779. Now hold the settlement through the winter until Donelson's river party arrives, April 24, 1780. A true founder!"
            }
        }
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
