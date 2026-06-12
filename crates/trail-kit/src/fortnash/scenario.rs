//! Fort Nash's scenario schema: the static description of the game's content,
//! parsed once from an embedded RON file and held immutable for the life of the
//! program. This is the *data* half of the data-driven engine — the tables that,
//! in the hand-written game, were `const` arrays and `match` arms: the trail
//! geography, the calendar, the random-event table, the minigame outcomes, the
//! scoring, and the ending text.

use serde::{Deserialize, Serialize};

use super::effect::Outcome;
use super::minigame::{MinigameParams, MinigameSpec};
use super::setpiece::Menus;

/// The complete static scenario. One per game, embedded and parsed once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    /// What the party sets out with and the outfitting rules.
    pub start: StartParams,
    /// The geography and calendar of the Wilderness Road.
    pub trail: TrailParams,
    /// Frontier-station economics.
    pub fort: FortParams,
    /// The winter calendar epoch.
    pub calendar: Calendar,
    /// Date at the start of each week, indexed by turn.
    pub dates: Vec<String>,
    /// The terrain bands, in trail order by milepost.
    pub checkpoints: Vec<Checkpoint>,
    /// The per-fortnight random-event roll.
    pub events: HazardTable,
    /// How a finished journey is scored and ranked.
    pub scoring: ScoringParams,
    /// The ending text, keyed by cause.
    pub endings: Vec<Ending>,
    /// The branching outcomes of each minigame hazard, keyed by id.
    pub outcomes: Vec<Outcome>,
    /// The launch parameters of each minigame, keyed by the same id.
    pub minigames: Vec<MinigameSpec<MinigameParams>>,
    /// The data-driven set-piece menus (Trail hub, Riders tactics).
    pub menus: Menus,
}

/// What the party sets out with and the outfitting rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartParams {
    /// Starting coin & trade goods.
    pub cash: f64,
    /// Livestock-train spend must fall in `oxen_min..=oxen_max`.
    pub oxen_min: f64,
    pub oxen_max: f64,
    /// Rounds bought per dollar of ammunition.
    pub bullets_per_dollar: f64,
    /// Rounds spent per shot fired in the hunting gallery.
    pub bullets_per_shot: f64,
}

/// The geography and calendar of the Wilderness Road.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrailParams {
    /// Total length of the road, on the stylized mileage scale.
    pub total_miles: f64,
    /// Mile past which the ridge country (and its passes) begins.
    pub mountains_at: f64,
    /// Mile of the Cumberland Gap (the second hard pass).
    pub cumberland_gap_at: f64,
    /// Mile of the frozen Cumberland River crossing (the Christmas crossing).
    pub cumberland_river_at: f64,
    /// Weeks past which the deep winter buries the party on the trail.
    pub max_turns: u32,
}

/// Frontier-station economics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FortParams {
    /// Miles lost detouring to a station / off to hunt.
    pub detour_miles: f64,
    /// Powder threshold (rounds) below which hunting is refused.
    pub hunt_min_bullets: f64,
    /// Goods bought at a station return `value_num / value_den` of their cost
    /// (frontier prices). A ratio, not a decimal, so the host's exact `2/3`
    /// arithmetic is preserved bit-for-bit.
    pub value_num: f64,
    pub value_den: f64,
}

/// The winter calendar epoch the journey is dated against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Calendar {
    /// Days per turn (a fortnight is 7 here on the stylized scale).
    pub period_days: i64,
    /// Day-of-year the party marches out (Nov 1 = 305).
    pub epoch_doy: i64,
    /// Weekday index of the start day (0 = Monday).
    pub epoch_weekday: i64,
    /// The year shown in dates.
    pub year: i64,
}

/// One terrain band, keyed by the mile at which it begins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub mile: f64,
    /// The "where you are" line shown on the trail hub.
    pub label: String,
    /// Cover-art slug (`trail-<key>`).
    pub key: String,
}

/// How a fortnight's event is selected from a single percentile roll, and what
/// each outcome is.
///
/// A roll `r1` in `0..100` selects `arms[i]` for the first threshold it falls
/// under (`r1 <= thresholds[i]`); if it clears every threshold, the final arm
/// (`arms[thresholds.len()]`) runs. So there is always exactly one more arm than
/// threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardTable {
    /// Cumulative percentile thresholds, ascending.
    pub thresholds: Vec<f64>,
    /// What each selected arm does; the last is the no-threshold-matched default.
    pub arms: Vec<HazardArm>,
}

impl HazardTable {
    /// The arm a percentile `roll` selects.
    pub fn select(&self, roll: f64) -> &HazardArm {
        for (i, t) in self.thresholds.iter().enumerate() {
            if roll <= *t {
                return &self.arms[i];
            }
        }
        &self.arms[self.thresholds.len()]
    }
}

/// What a selected event arm does — the data-driven event↔handler binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HazardArm {
    /// A constant-toll event: apply these effects immediately at `drift = 0`
    /// (the strays / lost-child / long-hunters events).
    Effects(Vec<super::effect::Effect>),
    /// Fire the minigame whose result selects `outcome` (an [`Outcome::id`]);
    /// the host resolver applies the tier when it resolves.
    Minigame { outcome: String },
    /// Run a built-in host handler by name (the RNG-bespoke events — bad water,
    /// the creek ford, the sleet storm — and the gunfight launches), whose
    /// internals keep their exact `self.rng` draws in the engine.
    Special(String),
    /// Pick one of two arms by whether the party is past the mountains.
    Branch {
        past_mountains: Box<HazardArm>,
        before: Box<HazardArm>,
    },
}

/// How a finished journey is scored and ranked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringParams {
    /// Flat reward for reaching the French Lick.
    pub base_win: f64,
    /// Days under which a speed bonus accrues.
    pub speed_par_days: i64,
    /// Speed bonus per day under par.
    pub speed_per_day: f64,
    /// On a loss, miles are divided by this for partial credit.
    pub loss_miles_div: f64,
    /// On a loss, leftover supply value is divided by this for partial credit.
    pub loss_leftover_div: f64,
    /// Rank tiers, highest threshold first.
    pub ranks: Vec<RankTier>,
    /// Rank awarded below the lowest tier.
    pub floor_rank: String,
}

/// One rank threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankTier {
    /// Minimum score to earn this rank.
    pub min: i64,
    pub name: String,
}

/// The text for one ending, keyed by its cause tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ending {
    /// Stable cause key (matches the host's `GameOverCause::key`).
    pub cause: String,
    pub message: String,
    pub won: bool,
}

impl Scenario {
    /// The ending whose cause key matches `key`, if any.
    pub fn ending(&self, key: &str) -> Option<&Ending> {
        self.endings.iter().find(|e| e.cause == key)
    }

    /// The outcome with the given id, if any.
    pub fn outcome(&self, id: &str) -> Option<&Outcome> {
        self.outcomes.iter().find(|o| o.id == id)
    }

    /// The launch parameters of the minigame with the given id, if any.
    pub fn minigame_params(&self, id: &str) -> Option<&MinigameParams> {
        self.minigames.iter().find(|m| m.id == id).map(|m| &m.params)
    }
}
