//! The scenario schema: the static description of a trail game's content, parsed
//! once from an embedded RON file and held immutable for the life of the program.
//!
//! This is the *data* half of the data-driven engine — the tables that, in a
//! hand-written game, would be `const` arrays and `match` arms: the goods hauled,
//! the boats, the river landings and their price ranks, the Trace stands, the
//! pace economy, scoring, and the ending text. The *flow* half (hazard tables,
//! branching outcomes, set-pieces) is layered on in later modules.

use serde::{Deserialize, Serialize};

use crate::effect::Outcome;
use crate::minigame::{MiniParams, MinigameSpec};
use crate::setpiece::Menus;

/// The complete static scenario. One per game, embedded and parsed once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    /// The tradeable goods, in canonical (index) order.
    pub goods: Vec<Good>,
    /// The boats that can be commissioned at the start.
    pub boats: Vec<BoatSpec>,
    /// Starting purse, credit, wages, and provisions.
    pub start: StartParams,
    /// Phase 1 — the downstream river run.
    pub river: RiverPhase,
    /// Phase 2 — the overland walk home.
    pub trace: TracePhase,
    /// How a finished journey is scored and ranked.
    pub scoring: ScoringParams,
    /// The set-piece economy and prose (the Falls, Natchez, stands, Duck River).
    pub setpieces: SetPieces,
    /// The set-piece menus (the buttons each set-piece screen shows).
    pub menus: Menus,
    /// The ending text, keyed by cause.
    pub endings: Vec<Ending>,
    /// The branching outcomes of each minigame hazard, keyed by id.
    pub outcomes: Vec<Outcome>,
    /// The launch parameters of each minigame, keyed by the same id.
    pub minigames: Vec<MinigameSpec<MiniParams>>,
    /// Ambient crew-banter pools, keyed by region. Optional: a scenario with no
    /// `banter:` block parses fine and quiet legs fall back to their flat line.
    #[serde(default)]
    pub banter: Vec<BanterPool>,
}

/// A region's worth of ambient crew banter. On a quiet (clean) leg whose
/// position falls in `[from_mile, to_mile)` of the matching `phase`, the engine
/// plays the first not-yet-heard [`BanterBeat`] whose gates pass — turning empty
/// travel into voiced world-building (geography, history, the people of the
/// country you're passing through). Selection is deterministic (never consumes
/// RNG), so it can't perturb hazard rolls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BanterPool {
    /// Which phase this region belongs to.
    pub phase: BanterPhase,
    /// Inclusive lower milepost bound (river miles, or Trace miles).
    pub from_mile: f64,
    /// Exclusive upper milepost bound.
    pub to_mile: f64,
    /// The beats for this region, tried in order; the first unheard, ungated one
    /// plays. Author most-specific or most-wanted first.
    pub beats: Vec<BanterBeat>,
}

/// Which travel phase a [`BanterPool`] applies to. Mirrors the engine's `Phase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BanterPhase {
    River,
    Trace,
}

/// One overheard exchange. Plays at most once per game (tracked by `key`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BanterBeat {
    /// Unique id; recorded in the run's heard-set so the beat never repeats.
    pub key: String,
    /// Optional state predicates; the beat is skipped unless all pass.
    #[serde(default)]
    pub gates: Vec<BanterGate>,
    /// The ordered lines of the exchange, surfaced one tap at a time.
    pub lines: Vec<BanterLine>,
}

/// A single spoken line: a speaker tag and what they say. The voice is free
/// text that prefixes the line (e.g. `"The old riverman spits:"`), so no UI
/// change or speaker registry is needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BanterLine {
    pub voice: String,
    pub text: String,
}

/// A state predicate gating a [`BanterBeat`], so tone can react to the journey
/// (a surly low-morale crew, company on the road). Evaluated against game state
/// by the host; no RNG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BanterGate {
    /// Morale strictly below this (0..100).
    MoraleBelow(f64),
    /// Morale at or above this (0..100).
    MoraleAbove(f64),
    /// Whether the party is travelling grouped.
    Grouped(bool),
    /// Whether a named antagonist has been marked as encountered (host-defined).
    /// Kaintuck uses it for Mason: set at Cave-in-Rock, read on the Trace.
    Marked(bool),
}

/// The set-piece numbers and single-line prose that are NOT already a menu
/// option's cost. Per-option prices (pilot fee, horse, rest) live once, on the
/// [`SetPieceOption`](crate::SetPieceOption) the player taps, and are charged
/// through the action's cost; only the non-menu values live here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetPieces {
    /// Toll the Duck River ferryman charges (a prompt, not a menu option).
    pub ferry_toll: f64,
    /// Provisions gained resting at a stand.
    pub rest_provisions: f64,
    /// Strength recovered resting at a stand.
    pub rest_health: f64,
    /// Narration when a pilot takes you down the Falls.
    pub falls_pilot_msg: String,
    /// Narration when the ferryman poles you over the Duck River.
    pub ferry_cross_msg: String,
    /// Narration when you cross the Duck River astride a horse.
    pub duck_horse_msg: String,
    /// Days lost reaching the ferry too late in the day to be set over (the host
    /// decides when "too late" applies). Defaults to 0 — no late penalty.
    #[serde(default)]
    pub ferry_late_days: u32,
    /// Narration when you reach the ferry after dark and wait for daylight.
    #[serde(default)]
    pub ferry_late_msg: String,
}

/// One tradeable good.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Good {
    pub name: String,
    /// Hold units one of this good occupies (livestock is bulky).
    pub units: i64,
}

/// A boat the player can commission, with its derived numbers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoatSpec {
    pub label: String,
    pub cost: f64,
    pub capacity: i64,
    /// How deep she sits — multiplies the sandbar/snag hazard.
    pub draft: f64,
    /// What her timbers fetch broken up for lumber at Natchez.
    pub lumber_value: f64,
    pub hint: String,
}

/// What the trader sets out with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartParams {
    pub cash: f64,
    pub credit_cap: f64,
    pub crew_wage: f64,
    pub provisions: f64,
}

/// Phase 1 — Pittsburgh to Natchez.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiverPhase {
    /// Total length of the downstream run, for the progress bar.
    pub river_miles: f64,
    /// The landings, in order from Pittsburgh.
    pub towns: Vec<Town>,
    /// The per-leg hazard roll.
    pub hazards: HazardTable,
}

/// How a leg's hazard is selected from a single percentile roll, and what each
/// outcome is.
///
/// A roll `r1` in `0..100` selects the first arm whose cumulative `thresholds`
/// entry it falls under; `arms[0]` is the clean leg (no threshold matched) and
/// `arms[i]` corresponds to `thresholds[i-1]` (so there is one more arm than
/// threshold). The selected [`HazardArm`] says, declaratively, which minigame
/// fires (by its outcome id) or which built-in handler runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardTable {
    /// Cumulative percentile thresholds, ascending.
    pub thresholds: Vec<f64>,
    /// Arm indices (1-based) that travelling in company can thin to a clean leg.
    pub grouped_thins: Vec<usize>,
    /// What each selected arm does. `arms[0]` is the clean leg.
    pub arms: Vec<HazardArm>,
}

/// What a selected hazard arm does — the data-driven minigame↔hazard binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HazardArm {
    /// Nothing happened; optionally narrate a quiet line.
    Clean {
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        cover: Option<String>,
    },
    /// Fire the minigame whose result selects `outcome` (an [`Outcome::id`]).
    /// An optional `message`/`cover` is narrated before the minigame begins.
    Minigame {
        outcome: String,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        cover: Option<String>,
    },
    /// Run a built-in host handler by name (e.g. cargo spoilage, crew grumbling)
    /// whose RNG-driven internals stay in the engine.
    Special(String),
    /// Pick one of two arms by whether the traveller is past the divide.
    Branch {
        past_divide: Box<HazardArm>,
        before: Box<HazardArm>,
    },
}

/// A river landing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Town {
    pub name: String,
    /// Cover-art slug (`town-<slug>`).
    pub slug: String,
    /// Cumulative river miles from Pittsburgh.
    pub milepost: f64,
    /// Base price rank per good (the mean quote is half this).
    pub base_ranks: Vec<i64>,
    /// Whether a generous moneylender here raises the credit cap.
    pub moneylender: bool,
    /// Hazard table for the leg *arriving at* this town, overriding the
    /// phase-wide [`RiverPhase::hazards`]. Keyed on the destination, so this is
    /// the override for Pittsburgh→here, not here→next; Pittsburgh's own entry
    /// is therefore never consulted (you never arrive at the start). Absent for
    /// most towns — only set where a leg's odds should differ historically (e.g.
    /// heavier piracy on the lower river toward Natchez).
    #[serde(default)]
    pub hazards: Option<HazardTable>,
}

/// Phase 2 — Natchez to Nashville.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TracePhase {
    /// The whole walk home, in miles.
    pub total_miles: f64,
    /// Past this many days, winter and exhaustion finish you.
    pub max_days: u32,
    /// Milepost past which the Harpe brothers' country begins.
    pub divide_at: f64,
    /// Milepost of the Duck River crossing.
    pub duck_river_at: f64,
    /// Miles-per-day multiplier when mounted.
    pub horse_multiplier: f64,
    /// The pace choices.
    pub paces: Vec<PaceSpec>,
    /// The stands, in trail order by milepost.
    pub stands: Vec<StandSpec>,
    /// The per-day hazard roll.
    pub hazards: HazardTable,
}

/// One travel pace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaceSpec {
    pub label: String,
    /// Miles covered in a day on foot, before the small random bonus.
    pub miles_per_day: f64,
    pub provisions_cost: f64,
    /// Wear on the body each day.
    pub health_cost: f64,
}

/// A stand or checkpoint along the Trace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandSpec {
    pub label: String,
    /// Cover-art slug (`trace-<key>`).
    pub key: String,
    pub milepost: f64,
    pub kind: StandKind,
    /// The "where you are" line shown on the stand screen.
    pub flavor: String,
}

/// What sort of stop a stand is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StandKind {
    /// A normal rest stand — pauses to its screen.
    Rest,
    /// A river crossing — resolved inline (horse, ferry, or ford).
    RiverCrossing,
}

/// How a finished journey is scored and ranked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringParams {
    /// Flat reward for arriving home.
    pub base_win: f64,
    /// Per surviving crew member.
    pub crew_bonus: f64,
    /// Reputation multiplier (clamped at `rep_floor`).
    pub rep_mult: f64,
    pub rep_floor: f64,
    /// Penalty if robbed on the Trace.
    pub robbed_penalty: f64,
    /// Days under which a speed bonus accrues.
    pub speed_par_days: i64,
    /// Speed bonus per day under par.
    pub speed_per_day: f64,
    /// On a loss, miles are divided by this for partial credit.
    pub loss_miles_div: f64,
    /// On a loss, leftover cash is divided by this for partial credit.
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
    pub fn minigame_params(&self, id: &str) -> Option<&MiniParams> {
        self.minigames.iter().find(|m| m.id == id).map(|m| &m.params)
    }
}
