//! Core data model: the duchy's people/land/grain, the King's mood, the soil
//! fertility tiers, and the screen modes.
//!
//! Field names track the canonical MS-BASIC / `imports/dukedom.c` globals where
//! it helps cross-checking the spec, but spelled out: `n_p` (nP, peasants),
//! `n_l` (nL, land in HA), `n_g` (nG, grain in HL), `n_y` (nY, year), `u1`/`u2`
//! (current / cumulative unrest), `k` (the King-state machine), `d` (black-plague
//! immunity countdown), `c1` (last year's effective yield — the land-price seed),
//! `c` (this year's yield). The 1-indexed ledger arrays `p`/`l`/`g` and the
//! fertility tiers `s` mirror the BASIC arrays (index 0 unused).

use serde::{Deserialize, Serialize};

/// Random spread half-width for `fnx` (BASIC `F3`).
pub const F3: i64 = 2;
/// Enemy-strength war multiplier (BASIC `M`).
pub const M: f64 = 1.95;

/// Starting peasants / land / grain.
pub const START_PEASANTS: i64 = 100;
pub const START_LAND: i64 = 600;
pub const START_GRAIN: i64 = 4177;
/// Initial land-price seed (BASIC `C1 = 3.95`).
pub const START_C1: f64 = 3.95;

/// Mandatory-retirement year: surviving past this (with the King appeased) wins.
pub const RETIREMENT_YEAR: i64 = 45;

/// Seeded "last year" ledger so the very first report shows a plausible founding
/// year (BASIC `numeric_data`). Layout: P[1..8], L[1..3], G[1..10], S[1..6].
pub const SEED_P: [i64; 8] = [96, 0, 0, 0, 0, 0, -4, 8];
pub const SEED_L: [i64; 3] = [600, 0, 0];
pub const SEED_G: [i64; 10] = [5193, -1344, 0, -768, 0, 0, 0, 1516, -120, -300];
pub const SEED_S: [i64; 6] = [216, 200, 184, 0, 0, 0];

/// Ledger line labels (BASIC `text_data`), 1-indexed via the helpers below.
pub const LABELS_P: [&str; 8] = [
    "Peasants at start",
    "Starvations",
    "King's levy",
    "War casualties",
    "Looting victims",
    "Disease victims",
    "Natural deaths",
    "Births",
];
pub const LABELS_L: [&str; 3] = ["Land at start", "Bought/sold", "Fruits of war"];
pub const LABELS_G: [&str; 10] = [
    "Grain at start",
    "Used for food",
    "Land deals",
    "Seedings",
    "Rat losses",
    "Mercenary hire",
    "Fruits of war",
    "Crop yield",
    "Castle expense",
    "Royal tax",
];

/// The duchy at one instant, plus the King/soil bookkeeping carried across years.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// The Duke's name (for the hall of fame).
    pub duke: String,
    /// Peasants (nP).
    pub n_p: i64,
    /// Land in hectares (nL).
    pub n_l: i64,
    /// Grain in hectoliters (nG).
    pub n_g: i64,
    /// Year (nY); the first played year is 1.
    pub n_y: i64,
    /// Current-year unrest (U1).
    pub u1: i64,
    /// Cumulative, decaying unrest (U2).
    pub u2: i64,
    /// King-state machine (K): 0 normal, 1 wary, 2 double-tax, -1 refused, -2 war.
    pub k: i64,
    /// Black-plague immunity countdown (D).
    pub d: i64,
    /// Last year's effective yield, the land-price seed (C1).
    pub c1: f64,
    /// This year's yield per HA, set at crop time, spent at harvest (C).
    pub c: f64,
    /// Soil fertility tiers S[1..6] = acres at 100/80/60/40/20%/depleted. Index 0
    /// is unused (kept for 1-based parity with the BASIC).
    pub s: [i64; 7],
    /// Scratch holding the cropped acres of each tier U[1..6] — written when the
    /// land tables update, read when the crop yield is computed (same year).
    pub u: [i64; 7],
    /// Per-game gauss event means R[1..8]. Index 0 unused.
    pub r: [i64; 9],
    /// Peasant ledger P[1..8] (this year's changes). Index 0 unused.
    pub p: [i64; 9],
    /// Land ledger L[1..3]. Index 0 unused.
    pub l: [i64; 4],
    /// Grain ledger G[1..10]. Index 0 unused.
    pub g: [i64; 11],
}

impl GameState {
    pub fn new(duke: String) -> Self {
        Self {
            duke,
            n_p: START_PEASANTS,
            n_l: START_LAND,
            n_g: START_GRAIN,
            n_y: 0,
            u1: 0,
            u2: 0,
            k: 0,
            d: 0,
            c1: START_C1,
            c: 0.0,
            s: [0; 7],
            u: [0; 7],
            r: [0; 9],
            p: [0; 9],
            l: [0; 4],
            g: [0; 11],
        }
    }

    /// Total "good" land — the top three fertility tiers, the only land buyers want.
    pub fn good_land(&self) -> i64 {
        self.s[1] + self.s[2] + self.s[3]
    }

    /// A short read on how restless the peasants are, for the status line.
    pub fn unrest_label(&self) -> &'static str {
        match self.u2.max(self.u1) {
            n if n > 80 => "Seething",
            n if n > 55 => "Restless",
            n if n > 30 => "Grumbling",
            _ => "Content",
        }
    }
}

/// Which screen is showing. Saved with the game so a refresh resumes in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Splash,
    /// Naming your Duke before the first year.
    NewGame,
    /// Start-of-year standings; tap to begin the year.
    YearReport,
    /// Allocating grain to feed the peasants over winter.
    Feed,
    /// Buying or selling land (a single transaction per year).
    Land,
    /// Choosing how many acres to plant.
    Plant,
    /// Showing the head of the pending message / decision queue.
    Interaction,
    GameOver,
}

/// How a reign ended — ordered worst-to-best in spirit; see `scoring`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ending {
    /// Defeated the King's army — you ARE the High King now.
    HighKing,
    /// Survived to mandatory retirement with the crown appeased.
    Retired,
    /// Deposed by your own restless, starving peasants.
    Deposed,
    /// Collapsed — too few peasants, too little land, or empty granaries.
    Collapsed,
    /// Lost a war (a rival Duke's, or the King's) — beheaded.
    Beheaded,
}

impl Ending {
    pub fn won(self) -> bool {
        matches!(self, Ending::HighKing | Ending::Retired)
    }
    pub fn label(self) -> &'static str {
        match self {
            Ending::HighKing => "Crowned High King",
            Ending::Retired => "Retired in glory",
            Ending::Deposed => "Deposed",
            Ending::Collapsed => "Duchy collapsed",
            Ending::Beheaded => "Beheaded",
        }
    }
}

/// The final reckoning, shown on the game-over screen and posted to the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGame {
    pub ending: Ending,
    /// One- or two-line narration of how it ended.
    pub cause: String,
    pub years: i64,
    pub peasants: i64,
    pub land: i64,
    pub grain: i64,
    pub score: i64,
    pub rank: String,
    pub recorded: bool,
}

/// A snapshot of the standings entering a year, with the prior year's ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YearReport {
    pub year: i64,
    pub peasants: i64,
    pub land: i64,
    pub grain: i64,
    /// Fertility tiers S[1..6] at report time.
    pub tiers: [i64; 7],
    /// Last year's peasant / land / grain ledgers (1-indexed).
    pub p: [i64; 9],
    pub l: [i64; 4],
    pub g: [i64; 11],
}

impl YearReport {
    /// Nonzero ledger lines with their labels, in report order (peasants, then
    /// land, then grain) — the mobile stand-in for the BASIC long report.
    pub fn ledger_lines(&self) -> Vec<(&'static str, i64)> {
        let mut out = Vec::new();
        for (i, label) in LABELS_P.iter().enumerate() {
            let v = self.p[i + 1];
            if v != 0 {
                out.push((*label, v));
            }
        }
        for (i, label) in LABELS_L.iter().enumerate() {
            let v = self.l[i + 1];
            if v != 0 {
                out.push((*label, v));
            }
        }
        for (i, label) in LABELS_G.iter().enumerate() {
            let v = self.g[i + 1];
            if v != 0 {
                out.push((*label, v));
            }
        }
        out
    }
}
