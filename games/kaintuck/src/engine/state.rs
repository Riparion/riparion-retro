//! Core data model: the trader's purse, the flatboat and its cargo, the walk up
//! the Trace, and the screen modes. Kaintuck is two games in one struct: a River
//! phase (Pittsburgh → Natchez) that is a downstream cargo-trading run, and a
//! Trace phase (Natchez → Nashville) that is a distance walk home on foot. One
//! [`Phase`] field gates which half is live; the [`Mode`] enum spans both.

use serde::{Deserialize, Serialize};

use super::scenario_data::scenario;

// ===== River phase =====

/// The seven things a Kaintuck hauls downriver, in canonical (index) order.
pub const NUM_GOODS: usize = 7;
pub const GOOD_NAMES: [&str; NUM_GOODS] = [
    "Corn", "Whiskey", "Flour", "Tobacco", "Pork", "Hides", "Livestock",
];
/// Hold units each good occupies. Livestock is bulky — two units a head.
pub const GOOD_UNITS: [i64; NUM_GOODS] = [1, 1, 1, 1, 1, 1, 2];

pub const NUM_RIVER_TOWNS: usize = 11;

pub const PITTSBURGH: usize = 0;
pub const CINCINNATI: usize = 4; // a generous moneylender
pub const LOUISVILLE: usize = 5; // the Falls of the Ohio
pub const CAIRO: usize = 7; // the Ohio–Mississippi confluence; Cave-in-Rock lies just above
pub const GRAND_TOWER: usize = 8; // the rivermen's initiation, on the Mississippi
pub const MEMPHIS: usize = 9; // a generous moneylender
pub const NATCHEZ: usize = 10;

/// Cover-art slugs per landing, in canonical town-index order.
pub const TOWN_SLUGS: [&str; NUM_RIVER_TOWNS] = [
    "pittsburgh",
    "wheeling",
    "marietta",
    "maysville",
    "cincinnati",
    "louisville",
    "shawneetown",
    "cairo",
    "grand-tower",
    "memphis",
    "natchez",
];


/// What kind of boat you commissioned in Pittsburgh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoatKind {
    Skiff,
    Flatboat,
    Broadhorn,
}

impl BoatKind {
    pub const ALL: [BoatKind; 3] = [BoatKind::Skiff, BoatKind::Flatboat, BoatKind::Broadhorn];

    /// Index into the scenario's boat table, in [`Self::ALL`] order.
    fn idx(self) -> usize {
        match self {
            BoatKind::Skiff => 0,
            BoatKind::Flatboat => 1,
            BoatKind::Broadhorn => 2,
        }
    }
    fn spec(self) -> &'static trail_kit::scenario::BoatSpec {
        &scenario().boats[self.idx()]
    }

    pub fn label(self) -> &'static str {
        self.spec().label.as_str()
    }
    pub fn cost(self) -> f64 {
        self.spec().cost
    }
    /// Hold capacity in units.
    pub fn capacity(self) -> i64 {
        self.spec().capacity
    }
    /// How deep she sits — multiplies the sandbar/snag hazard on the river.
    pub fn draft(self) -> f64 {
        self.spec().draft
    }
    /// What her timbers fetch broken up for lumber at Natchez.
    pub fn lumber_value(self) -> f64 {
        self.spec().lumber_value
    }
    pub fn hint(self) -> &'static str {
        self.spec().hint.as_str()
    }
}

// ===== Trace phase =====

/// How hard you push on the Trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pace {
    Steady,
    Hard,
}

impl Pace {
    /// Index into the scenario's pace table.
    fn idx(self) -> usize {
        match self {
            Pace::Steady => 0,
            Pace::Hard => 1,
        }
    }
    fn spec(self) -> &'static trail_kit::scenario::PaceSpec {
        &scenario().trace.paces[self.idx()]
    }
    /// Miles covered in a day, before the day's small random bonus.
    pub fn miles_per_day(self, horse: bool) -> f64 {
        let base = self.spec().miles_per_day;
        if horse {
            base * scenario().trace.horse_multiplier
        } else {
            base
        }
    }
    pub fn provisions_cost(self) -> f64 {
        self.spec().provisions_cost
    }
    /// Wear on the body each day — pushing hard grinds you down.
    pub fn health_cost(self) -> f64 {
        self.spec().health_cost
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
    /// The stands in trail order.
    pub const ALL: [Stand; 5] = [
        Stand::MountLocust,
        Stand::Choctaw,
        Stand::BuzzardRoost,
        Stand::TnDivide,
        Stand::DuckRiver,
    ];

    /// Index into the scenario's stand table, in [`Self::ALL`] order.
    fn idx(self) -> usize {
        match self {
            Stand::MountLocust => 0,
            Stand::Choctaw => 1,
            Stand::BuzzardRoost => 2,
            Stand::TnDivide => 3,
            Stand::DuckRiver => 4,
        }
    }
    fn spec(self) -> &'static trail_kit::scenario::StandSpec {
        &scenario().trace.stands[self.idx()]
    }

    /// This stand's milepost, off the scenario.
    pub fn milepost(self) -> f64 {
        self.spec().milepost
    }

    /// Stands in trail order, paired with their mileposts — the old `POSTS`
    /// const, now sourced from the scenario.
    pub fn posts() -> [(Stand, f64); 5] {
        let mut out = [(Stand::MountLocust, 0.0); 5];
        for (i, &s) in Stand::ALL.iter().enumerate() {
            out[i] = (s, s.milepost());
        }
        out
    }

    pub fn label(self) -> &'static str {
        self.spec().label.as_str()
    }
    pub fn key(self) -> &'static str {
        self.spec().key.as_str()
    }
    /// The "where you are" line for the stand screen.
    pub fn flavor(self) -> &'static str {
        self.spec().flavor.as_str()
    }
    /// The "where you are now" line for the Trace hub, by milepost. Thresholds
    /// are read off [`Stand::POSTS`] so the location text can't drift from the
    /// mileposts that actually trigger each stop.
    pub fn current(miles: f64) -> &'static str {
        let p = Self::posts();
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
        } else if miles < scenario().trace.total_miles {
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
    pub town: usize,
    pub credit_cap: f64,
    // --- Trace walk ---
    pub miles: f64,
    pub provisions: f64,
    pub health: f64,
    /// Days lost to deliberate dawdling (convoy waits, a late ferry) on top of
    /// the river-mile and Trace-day reckoning. Folds into the speed score.
    /// `serde(default)` keeps pre-convoy saves loading.
    #[serde(default)]
    pub extra_days: u32,
    pub has_horse: bool,
    pub pace: Pace,
    pub grouped: bool,
    /// River-side analogue of `grouped`: the boat runs the lower river in a
    /// convoy ("sailing in company"), thinning pirate odds and halving their
    /// take. `serde(default)` keeps pre-convoy saves loading.
    #[serde(default)]
    pub river_convoy: bool,
    pub stand_idx: usize,
    pub day: u32,
    // --- shared / scoring ---
    pub reputation: f64,
    pub robbed: bool,
    /// You tangled with Mason's river gang at Cave-in-Rock — i.e. you took the
    /// stranger's offer with a hold worth robbing and were boarded (the pirate
    /// quick-draw). Set only on that confrontation, not on merely passing the
    /// cave. Persists into the Trace (Phase 2) to gate the wanted-poster payoff.
    /// `serde(default)` keeps older saves loading.
    #[serde(default)]
    pub crossed_mason: bool,
    pub crew_at_natchez: i64,
    /// Keys of ambient banter beats already heard this run, so each plays once.
    /// `serde(default)` keeps pre-banter saves loading (they resume with none).
    #[serde(default)]
    pub heard_banter: std::collections::HashSet<String>,
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
            cash: scenario().start.cash,
            debt: 0.0,
            boat: None,
            hold: [0; NUM_GOODS],
            crew: 0,
            morale: 100.0,
            prices: [0.0; NUM_GOODS],
            town: PITTSBURGH,
            credit_cap: scenario().start.credit_cap,
            miles: 0.0,
            provisions: 0.0,
            health: 100.0,
            extra_days: 0,
            has_horse: false,
            pace: Pace::Steady,
            grouped: false,
            river_convoy: false,
            stand_idx: 0,
            day: 0,
            reputation: 0.0,
            robbed: false,
            crossed_mason: false,
            crew_at_natchez: 0,
            heard_banter: std::collections::HashSet::new(),
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
        scenario().river.towns[self.town.min(NUM_RIVER_TOWNS - 1)].milepost
    }

    pub fn town_name(&self) -> &'static str {
        scenario().river.towns[self.town.min(NUM_RIVER_TOWNS - 1)]
            .name
            .as_str()
    }

    /// Total miles of the whole journey covered so far (river + trace), for the
    /// final reckoning and partial-credit scoring.
    pub fn total_miles(&self, phase: Phase) -> f64 {
        match phase {
            Phase::River => self.river_mile(),
            Phase::Trace => {
                scenario().river.river_miles + self.miles.min(scenario().trace.total_miles)
            }
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
    /// The market: an actionable table where you buy and/or sell cargo in place.
    /// `can_buy`/`can_sell` gate the columns and actions per hub (Pittsburgh buys
    /// only, Natchez sells only, river towns do both).
    Trade { can_buy: bool, can_sell: bool },
    /// Borrow against the boat or repay the boatyard.
    Moneylender,
    /// The Falls of the Ohio — portage, pilot, or run them.
    Falls,
    /// Grand Tower on the Mississippi — the rivermen's initiation: treat or ducking.
    GrandTower,
    /// Cave-in-Rock, on the run down to Cairo — the relay-pilot con: take the
    /// stranger's offer, hire your own pilot, or run the shoals.
    CaveInRock,
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
    Heave,
    HotCold,
    Hunter,
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
    /// The cause for a key (the inverse of [`Self::key`]); for data-driven
    /// `Die`/`DieIfDead` effects that name their cause by string.
    pub fn from_key(key: &str) -> Option<GameOverCause> {
        match key {
            "boat-wrecked" => Some(GameOverCause::BoatWrecked),
            "drowned" => Some(GameOverCause::Drowned),
            "bandit-murder" => Some(GameOverCause::BanditMurder),
            "disease" => Some(GameOverCause::Disease),
            "starved" => Some(GameOverCause::Starved),
            "lost-in-woods" => Some(GameOverCause::LostInWoods),
            "victory" => Some(GameOverCause::Victory),
            _ => None,
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
        assert_eq!(s.cash, scenario().start.cash);
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
        let towns = &scenario().river.towns;
        let pgh = towns[PITTSBURGH].base_ranks[1];
        let nat = towns[NATCHEZ].base_ranks[1];
        assert!(nat > pgh * 2, "whiskey should more than double by Natchez");
    }

    #[test]
    fn stand_posts_are_in_order() {
        let posts = Stand::posts();
        for w in posts.windows(2) {
            assert!(w[0].1 < w[1].1);
        }
    }
}
