//! Core data model: the trader's purse, the flatboat and its cargo, the walk up
//! the Trace, and the screen modes. Kaintuck is two games in one struct: a River
//! phase (Pittsburgh → Natchez) that is a downstream cargo-trading run, and a
//! Trace phase (Natchez → Nashville) that is a distance walk home on foot. One
//! [`Phase`] field gates which half is live; the [`Mode`] enum spans both.

use serde::{Deserialize, Serialize};

// ===== River phase =====

/// The seven things a Kaintuck hauls downriver, in canonical (index) order.
pub const NUM_GOODS: usize = 7;
pub const GOOD_NAMES: [&str; NUM_GOODS] = [
    "Corn", "Whiskey", "Flour", "Tobacco", "Pork", "Hides", "Livestock",
];
/// Hold units each good occupies. Livestock is bulky — two units a head.
pub const GOOD_UNITS: [i64; NUM_GOODS] = [1, 1, 1, 1, 1, 1, 2];

pub const NUM_RIVER_TOWNS: usize = 7;
pub const TOWN_NAMES: [&str; NUM_RIVER_TOWNS] = [
    "Pittsburgh",
    "Wheeling",
    "Cincinnati",
    "Louisville",
    "Cairo",
    "Memphis",
    "Natchez",
];
/// Cumulative river miles from Pittsburgh to each landing.
pub const RIVER_MILEPOSTS: [f64; NUM_RIVER_TOWNS] = [
    0.0, 90.0, 470.0, 600.0, 980.0, 1430.0, 1830.0,
];
/// Total length of the downstream run, for the progress bar.
pub const RIVER_MILES: f64 = 1830.0;

pub const PITTSBURGH: usize = 0;
pub const CINCINNATI: usize = 2; // a generous moneylender
pub const LOUISVILLE: usize = 3; // the Falls of the Ohio
pub const MEMPHIS: usize = 5; // a generous moneylender
pub const NATCHEZ: usize = 6;

/// Cover-art slugs per landing, kept beside `TOWN_NAMES` so the two can't drift.
pub const TOWN_SLUGS: [&str; NUM_RIVER_TOWNS] = [
    "pittsburgh",
    "wheeling",
    "cincinnati",
    "louisville",
    "cairo",
    "memphis",
    "natchez",
];

/// Base price ranks `[town][good]` — roughly the mean dollar price a good fetches
/// at each landing (the `(R(3)+1)` jitter spreads each quote 1×–3× of half this).
/// Tuned so goods bought cheap in Pittsburgh sell progressively higher downstream
/// — whiskey and tobacco climb most toward Natchez; livestock peaks mid-river and
/// loses value in the hot south.
pub const INITIAL_BASE_RANKS: [[i64; NUM_GOODS]; NUM_RIVER_TOWNS] = [
    // Corn Whk Flr Tob Prk Hid Liv
    [2, 8, 4, 6, 5, 7, 12],   // Pittsburgh (the cheap upstream buy market)
    [2, 10, 4, 7, 6, 7, 11],  // Wheeling
    [3, 13, 6, 9, 7, 8, 17],  // Cincinnati
    [3, 14, 6, 10, 8, 9, 18], // Louisville
    [3, 15, 6, 11, 8, 10, 19], // Cairo
    [3, 16, 6, 11, 9, 11, 19], // Memphis
    [4, 21, 8, 14, 11, 15, 18], // Natchez
];

/// You start with little but ambition; the boatyard will extend credit.
pub const STARTING_CASH: f64 = 50.0;
/// Most the boatyard will let you owe when you set out.
pub const STARTING_CREDIT_CAP: f64 = 200.0;
/// Wage paid up front per hand hired.
pub const CREW_WAGE: f64 = 3.0;

/// What kind of boat you commissioned in Pittsburgh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoatKind {
    Skiff,
    Flatboat,
    Broadhorn,
}

impl BoatKind {
    pub const ALL: [BoatKind; 3] = [BoatKind::Skiff, BoatKind::Flatboat, BoatKind::Broadhorn];

    pub fn label(self) -> &'static str {
        match self {
            BoatKind::Skiff => "Skiff",
            BoatKind::Flatboat => "Flatboat",
            BoatKind::Broadhorn => "Broadhorn",
        }
    }
    pub fn cost(self) -> f64 {
        match self {
            BoatKind::Skiff => 40.0,
            BoatKind::Flatboat => 75.0,
            BoatKind::Broadhorn => 130.0,
        }
    }
    /// Hold capacity in units.
    pub fn capacity(self) -> i64 {
        match self {
            BoatKind::Skiff => 30,
            BoatKind::Flatboat => 60,
            BoatKind::Broadhorn => 100,
        }
    }
    /// How deep she sits — multiplies the sandbar/snag hazard on the river.
    pub fn draft(self) -> f64 {
        match self {
            BoatKind::Skiff => 0.7,
            BoatKind::Flatboat => 1.0,
            BoatKind::Broadhorn => 1.4,
        }
    }
    /// What her timbers fetch broken up for lumber at Natchez.
    pub fn lumber_value(self) -> f64 {
        match self {
            BoatKind::Skiff => 25.0,
            BoatKind::Flatboat => 55.0,
            BoatKind::Broadhorn => 95.0,
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            BoatKind::Skiff => "cheap, light, shallow — small load",
            BoatKind::Flatboat => "the workhorse of the river",
            BoatKind::Broadhorn => "hauls a fortune — and draws deep water",
        }
    }
}

// ===== Trace phase =====

/// The whole walk home, in miles.
pub const TRACE_MILES: f64 = 450.0;
/// Past this many days on the Trace, winter and exhaustion finish you.
pub const MAX_DAYS: u32 = 40;
/// Provisions packed for the walk when you set out from Natchez.
pub const STARTING_PROVISIONS: f64 = 50.0;
/// Milepost past which the Harpe brothers' country begins (TN Valley Divide).
pub const DIVIDE_AT: f64 = 423.9;
/// Milepost of the Duck River crossing.
pub const DUCK_RIVER_AT: f64 = 440.0;

/// How hard you push on the Trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pace {
    Steady,
    Hard,
}

impl Pace {
    /// Miles covered in a day, before the day's small random bonus.
    pub fn miles_per_day(self, horse: bool) -> f64 {
        let base = match self {
            Pace::Steady => 14.0,
            Pace::Hard => 20.0,
        };
        if horse {
            base * 1.6
        } else {
            base
        }
    }
    pub fn provisions_cost(self) -> f64 {
        match self {
            Pace::Steady => 4.0,
            Pace::Hard => 6.0,
        }
    }
    /// Wear on the body each day — pushing hard grinds you down.
    pub fn health_cost(self) -> f64 {
        match self {
            Pace::Steady => 1.0,
            Pace::Hard => 4.0,
        }
    }
}

/// The stands and checkpoints along the Trace, by milepost. `key` is the
/// cover-art slug (`trace-<key>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stand {
    MountLocust,
    Choctaw,
    BuzzardRoost,
    TnDivide,
    DuckRiver,
}

impl Stand {
    /// Stands in trail order, with their mileposts.
    pub const POSTS: [(Stand, f64); 5] = [
        (Stand::MountLocust, 15.5),
        (Stand::Choctaw, 120.0),
        (Stand::BuzzardRoost, 320.3),
        (Stand::TnDivide, DIVIDE_AT),
        (Stand::DuckRiver, DUCK_RIVER_AT),
    ];

    pub fn label(self) -> &'static str {
        match self {
            Stand::MountLocust => "Mount Locust",
            Stand::Choctaw => "The Choctaw Territory",
            Stand::BuzzardRoost => "Buzzard Roost Spring",
            Stand::TnDivide => "Tennessee Valley Divide",
            Stand::DuckRiver => "Duck River Crossing",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Stand::MountLocust => "mount-locust",
            Stand::Choctaw => "choctaw",
            Stand::BuzzardRoost => "buzzard-roost",
            Stand::TnDivide => "tn-divide",
            Stand::DuckRiver => "duck-river",
        }
    }
    /// The "where you are now" line for the Trace hub, by milepost. Thresholds
    /// are read off [`Stand::POSTS`] so the location text can't drift from the
    /// mileposts that actually trigger each stop.
    pub fn current(miles: f64) -> &'static str {
        let p = Self::POSTS;
        if miles < p[0].1 {
            "The road north out of Natchez"
        } else if miles < p[1].1 {
            "Mount Locust country"
        } else if miles < p[2].1 {
            "Deep in the Choctaw woods"
        } else if miles < p[3].1 {
            "Chickasaw land — Buzzard Roost behind you"
        } else if miles < p[4].1 {
            "Past the divide — Harpe country"
        } else if miles < TRACE_MILES {
            "The Tennessee hills"
        } else {
            "Nashville"
        }
    }
}

// ===== The world =====

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub trader: String,
    // --- money (both phases) ---
    pub cash: f64,
    pub debt: f64,
    // --- River trade ---
    pub boat: Option<Boat>,
    pub hold: [i64; NUM_GOODS],
    pub crew: i64,
    pub morale: f64,
    pub prices: [f64; NUM_GOODS],
    pub base_ranks: [[i64; NUM_GOODS]; NUM_RIVER_TOWNS],
    pub town: usize,
    pub credit_cap: f64,
    // --- Trace walk ---
    pub miles: f64,
    pub provisions: f64,
    pub health: f64,
    pub has_horse: bool,
    pub pace: Pace,
    pub grouped: bool,
    pub stand_idx: usize,
    pub day: u32,
    // --- shared / scoring ---
    pub reputation: f64,
    pub robbed: bool,
    pub crew_at_natchez: i64,
}

/// The flatboat. Only the `kind` is stored; the derived numbers are methods that
/// delegate to it, so a rebalance of [`BoatKind`] can never leave a save carrying
/// a stale cached capacity/draft/lumber value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Boat {
    pub kind: BoatKind,
}

impl Boat {
    pub fn new(kind: BoatKind) -> Self {
        Self { kind }
    }
    pub fn capacity(self) -> i64 {
        self.kind.capacity()
    }
    pub fn draft(self) -> f64 {
        self.kind.draft()
    }
    pub fn lumber_value(self) -> f64 {
        self.kind.lumber_value()
    }
}

impl GameState {
    pub fn new(trader: String) -> Self {
        Self {
            trader,
            cash: STARTING_CASH,
            debt: 0.0,
            boat: None,
            hold: [0; NUM_GOODS],
            crew: 0,
            morale: 100.0,
            prices: [0.0; NUM_GOODS],
            base_ranks: INITIAL_BASE_RANKS,
            town: PITTSBURGH,
            credit_cap: STARTING_CREDIT_CAP,
            miles: 0.0,
            provisions: 0.0,
            health: 100.0,
            has_horse: false,
            pace: Pace::Steady,
            grouped: false,
            stand_idx: 0,
            day: 0,
            reputation: 0.0,
            robbed: false,
            crew_at_natchez: 0,
        }
    }

    pub fn capacity(&self) -> i64 {
        self.boat.map(|b| b.capacity()).unwrap_or(0)
    }

    /// Hold units occupied by cargo (livestock counts double).
    pub fn cargo_units(&self) -> i64 {
        self.hold
            .iter()
            .zip(GOOD_UNITS)
            .map(|(&n, u)| n * u)
            .sum()
    }

    /// Free hold units. Negative means overloaded.
    pub fn free_hold(&self) -> i64 {
        self.capacity() - self.cargo_units()
    }

    /// Market value of the cargo aboard at the current prices.
    pub fn cargo_value(&self) -> f64 {
        self.hold
            .iter()
            .zip(self.prices)
            .map(|(&n, p)| n as f64 * p)
            .sum()
    }

    pub fn river_mile(&self) -> f64 {
        RIVER_MILEPOSTS[self.town.min(NUM_RIVER_TOWNS - 1)]
    }

    pub fn town_name(&self) -> &'static str {
        TOWN_NAMES[self.town.min(NUM_RIVER_TOWNS - 1)]
    }

    /// Total miles of the whole journey covered so far (river + trace), for the
    /// final reckoning and partial-credit scoring.
    pub fn total_miles(&self, phase: Phase) -> f64 {
        match phase {
            Phase::River => self.river_mile(),
            Phase::Trace => RIVER_MILES + self.miles.min(TRACE_MILES),
        }
    }

    pub fn morale_label(&self) -> &'static str {
        match self.morale {
            m if m >= 80.0 => "High",
            m if m >= 50.0 => "Steady",
            m if m >= 25.0 => "Low",
            _ => "Mutinous",
        }
    }

    pub fn health_label(&self) -> &'static str {
        match self.health {
            h if h >= 75.0 => "Hale",
            h if h >= 45.0 => "Worn",
            h if h >= 20.0 => "Ailing",
            _ => "Failing",
        }
    }
}

/// Which screen is showing. Saved with the game so a refresh resumes in place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    Splash,
    NewGame,
    // --- River ---
    /// Build & outfit at Pittsburgh: boat, crew, then load cargo.
    Pittsburgh,
    /// A river-town hub: trade, see the moneylender, cast off.
    Town,
    /// Buy (true) or sell (false) cargo.
    Trade { buying: bool },
    /// Borrow against the boat or repay the boatyard.
    Moneylender,
    /// The Falls of the Ohio — portage, pilot, or run them.
    Falls,
    /// Natchez: sell everything, the boat for lumber, and a night Under-the-Hill.
    Natchez,
    // --- Trace ---
    /// The day's hub: pace, company, press on.
    TraceHub,
    /// Resting and resupplying at a stand.
    Stand,
    // --- minigames (the pending_* task says which) ---
    Steady,
    Quick,
    Crowd,
    Timing,
    Sequence,
    Brigade,
    // --- shared ---
    Interaction,
    GameOver,
}

/// Which leg of the journey is underway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    River,
    Trace,
}

/// Why the journey ended — a stable tag for each death and the victory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameOverCause {
    /// The boat broke up — the Falls, or a snag taken badly.
    BoatWrecked,
    /// Drowned at a river crossing on the Trace.
    Drowned,
    /// Killed in an ambush on the Trace.
    BanditMurder,
    /// Fever or exhaustion took you on the Trace.
    Disease,
    /// Out of provisions, you starved on the Trace.
    Starved,
    /// Winter caught you still on the Trace.
    LostInWoods,
    Victory,
}

impl GameOverCause {
    pub fn won(self) -> bool {
        matches!(self, GameOverCause::Victory)
    }
    pub fn key(self) -> &'static str {
        match self {
            GameOverCause::BoatWrecked => "boat-wrecked",
            GameOverCause::Drowned => "drowned",
            GameOverCause::BanditMurder => "bandit-murder",
            GameOverCause::Disease => "disease",
            GameOverCause::Starved => "starved",
            GameOverCause::LostInWoods => "lost-in-woods",
            GameOverCause::Victory => "victory",
        }
    }
    pub fn message(self) -> &'static str {
        match self {
            GameOverCause::BoatWrecked => {
                "Your flatboat broke apart and went down with everything aboard."
            }
            GameOverCause::Drowned => "You drowned trying to cross a flooded river on the Trace.",
            GameOverCause::BanditMurder => "Bandits cut you down on the Trace and took everything.",
            GameOverCause::Disease => "Fever and exhaustion took you on the long walk home.",
            GameOverCause::Starved => "Your provisions ran out and you starved in the wilderness.",
            GameOverCause::LostInWoods => {
                "Winter caught you still on the Trace, far from Nashville."
            }
            GameOverCause::Victory => {
                "You walked into Nashville with your earnings — a Kaintuck come home rich and alive."
            }
        }
    }
}

/// How the journey ended, with the final reckoning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGame {
    pub won: bool,
    pub cause: String,
    pub cause_kind: GameOverCause,
    pub score: i64,
    pub rank: String,
    pub cash: i64,
    pub debt: i64,
    pub crew_survived: i64,
    pub reputation: i64,
    pub robbed: bool,
    pub miles: i64,
    pub days: i64,
    pub recorded: bool,
}

// Money formatting lives in retro-kit; re-export so engine/UI share the path.
pub use retro_kit::format::fmt_money;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_state_is_consistent() {
        let s = GameState::new("Tony".into());
        assert_eq!(s.cash, STARTING_CASH);
        assert_eq!(s.capacity(), 0);
        assert_eq!(s.free_hold(), 0);
        assert_eq!(s.town_name(), "Pittsburgh");
        assert_eq!(s.total_miles(Phase::River), 0.0);
    }

    #[test]
    fn boat_units_and_capacity() {
        let mut s = GameState::new("T".into());
        s.boat = Some(Boat::new(BoatKind::Flatboat));
        assert_eq!(s.capacity(), 60);
        s.hold[6] = 5; // 5 livestock = 10 units
        assert_eq!(s.cargo_units(), 10);
        s.hold[0] = 20; // +20 corn = 20 units
        assert_eq!(s.cargo_units(), 30);
        assert_eq!(s.free_hold(), 30);
    }

    #[test]
    fn natchez_ranks_top_the_market_for_whiskey() {
        let pgh = INITIAL_BASE_RANKS[PITTSBURGH][1];
        let nat = INITIAL_BASE_RANKS[NATCHEZ][1];
        assert!(nat > pgh * 2, "whiskey should more than double by Natchez");
    }

    #[test]
    fn stand_posts_are_in_order() {
        let posts = Stand::POSTS;
        for w in posts.windows(2) {
            assert!(w[0].1 < w[1].1);
        }
    }
}
