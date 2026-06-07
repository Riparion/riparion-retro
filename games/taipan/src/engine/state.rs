//! Core game data model: ports, commodities, the player/world state, and screen modes.

use serde::{Deserialize, Serialize};

pub const HONG_KONG: usize = 1;
pub const AT_SEA: usize = 0;
pub const NUM_PORTS: usize = 7;
pub const WAREHOUSE_CAPACITY: i64 = 10_000;
pub const GUN_HOLD_UNITS: i64 = 10;

pub const PORT_NAMES: [&str; 8] = [
    "At sea",
    "Hong Kong",
    "Shanghai",
    "Nagasaki",
    "Saigon",
    "Manila",
    "Singapore",
    "Batavia",
];

pub const ITEM_NAMES: [&str; 4] = ["Opium", "Silk", "Arms", "General Cargo"];

pub const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Initial base price ranks `BP%[port][item]` from the BASIC source.
/// Rows are ports 1..=7 (Hong Kong..Batavia), columns are Opium/Silk/Arms/General.
pub const INITIAL_BASE_RANKS: [[i64; 4]; NUM_PORTS] = [
    [11, 11, 12, 10], // Hong Kong
    [16, 14, 16, 11], // Shanghai
    [15, 15, 10, 12], // Nagasaki
    [14, 16, 11, 13], // Saigon
    [12, 10, 13, 14], // Manila
    [10, 13, 14, 15], // Singapore
    [13, 12, 15, 16], // Batavia
];

/// How the player chose to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartChoice {
    /// Cash 400, debt 5000, no guns. Pirates 1 in 10.
    Cash,
    /// Five guns, no cash, no debt. Pirates 1 in 7.
    Guns,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub firm: String,
    pub cash: f64,
    pub bank: f64,
    pub debt: f64,
    /// Ship capacity in hold units (`SC`).
    pub capacity: i64,
    pub guns: i64,
    /// Accumulated hull damage (`DM`); seaworthiness = 100 - damage/capacity*100.
    pub damage: f64,
    /// Cargo aboard ship, indexed by commodity.
    pub hold: [i64; 4],
    /// Cargo in the Hong Kong godown.
    pub warehouse: [i64; 4],
    /// Current location: 0 = at sea, 1..=7 = port index.
    pub port: usize,
    pub month: u32,
    pub year: u32,
    /// Months elapsed (`TI`), starts at 1.
    pub months: u32,
    /// Drifting base price ranks (`BP%`), inflated yearly.
    pub base_ranks: [[i64; 4]; NUM_PORTS],
    /// Current prices at this port (`CP`).
    pub prices: [f64; 4],
    /// Li Yuen protection active (`LI`).
    pub li_yuen: bool,
    /// Pirate rarity divisor (`BP`): 10 for cash start, 7 for guns start.
    pub pirate_divisor: f64,
    /// Enemy ship health base (`EC`), +10 per year.
    pub ec: f64,
    /// Enemy damage base (`ED`), +0.5 per year.
    pub ed: f64,
    /// Wu kidnapping warning already shown (`WN`).
    pub wu_warned: bool,
    /// Number of times Wu has bailed the player out (`BL%`).
    pub bailouts: u32,
}

impl GameState {
    pub fn new(firm: String, choice: StartChoice) -> Self {
        let (cash, debt, guns, pirate_divisor) = match choice {
            StartChoice::Cash => (400.0, 5000.0, 0, 10.0),
            StartChoice::Guns => (0.0, 0.0, 5, 7.0),
        };
        Self {
            firm,
            cash,
            bank: 0.0,
            debt,
            capacity: 60,
            guns,
            damage: 0.0,
            hold: [0; 4],
            warehouse: [0; 4],
            port: HONG_KONG,
            month: 1,
            year: 1860,
            months: 1,
            base_ranks: INITIAL_BASE_RANKS,
            prices: [0.0; 4],
            li_yuen: false,
            pirate_divisor,
            ec: 20.0,
            ed: 0.5,
            wu_warned: false,
            bailouts: 0,
        }
    }

    pub fn cargo_aboard(&self) -> i64 {
        self.hold.iter().sum()
    }

    pub fn warehouse_used(&self) -> i64 {
        self.warehouse.iter().sum()
    }

    /// Free hold space: capacity − cargo − 10 per gun. Can go negative (overload).
    pub fn free_hold(&self) -> i64 {
        self.capacity - self.cargo_aboard() - GUN_HOLD_UNITS * self.guns
    }

    pub fn net_worth(&self) -> f64 {
        self.cash + self.bank - self.debt
    }

    /// 0..=100; 0 means the ship is lost.
    pub fn seaworthiness(&self) -> i64 {
        (100 - (self.damage / self.capacity as f64 * 100.0).floor() as i64).max(0)
    }

    /// Critical/Poor/Fair/Good/Prime/Perfect by 20% band.
    pub fn seaworthiness_title(&self) -> &'static str {
        const TITLES: [&str; 6] = ["Critical", "Poor", "Fair", "Good", "Prime", "Perfect"];
        let sw = 100 - (self.damage / self.capacity as f64 * 100.0 + 0.5).floor() as i64;
        let sw = sw.clamp(0, 100);
        TITLES[(sw / 20).min(5) as usize]
    }

    pub fn date_string(&self) -> String {
        format!("{} {}", MONTH_NAMES[(self.month as usize - 1) % 12], self.year)
    }

    pub fn port_name(&self) -> &'static str {
        PORT_NAMES[self.port]
    }
}

/// Which screen the UI is showing. Part of the saved game so a refresh resumes in place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    Splash,
    NewGame,
    Port,
    /// Buy (true) or sell (false) flow.
    Trade { buying: bool },
    Transfer,
    Bank,
    Wu,
    Travel,
    Combat,
    /// Presenting the head of the pending interaction queue.
    Interaction,
    GameOver,
}

/// Why the game ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGame {
    pub retired: bool,
    pub cause: String,
    pub net_worth: f64,
    pub score: i64,
    pub rank: String,
    pub years: u32,
    pub months: u32,
    pub capacity: i64,
    pub guns: i64,
    pub recorded: bool,
}

// Money formatting lives in retro-kit; re-exported here so engine and UI
// call sites keep their `state::fmt_money` path.
pub use retro_kit::format::fmt_money;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_choices_match_original() {
        let g = GameState::new("Test".into(), StartChoice::Cash);
        assert_eq!(g.cash, 400.0);
        assert_eq!(g.debt, 5000.0);
        assert_eq!(g.guns, 0);
        assert_eq!(g.free_hold(), 60);
        assert_eq!(g.pirate_divisor, 10.0);

        let g = GameState::new("Test".into(), StartChoice::Guns);
        assert_eq!(g.cash, 0.0);
        assert_eq!(g.debt, 0.0);
        assert_eq!(g.guns, 5);
        assert_eq!(g.free_hold(), 10);
        assert_eq!(g.pirate_divisor, 7.0);
        assert_eq!(g.port, HONG_KONG);
        assert_eq!(g.date_string(), "Jan 1860");
    }

    #[test]
    fn seaworthiness_bands() {
        let mut g = GameState::new("T".into(), StartChoice::Cash);
        assert_eq!(g.seaworthiness(), 100);
        assert_eq!(g.seaworthiness_title(), "Perfect");
        g.damage = 30.0; // half of 60 capacity
        assert_eq!(g.seaworthiness(), 50);
        assert_eq!(g.seaworthiness_title(), "Fair");
        g.damage = 60.0;
        assert_eq!(g.seaworthiness(), 0);
        assert_eq!(g.seaworthiness_title(), "Critical");
    }

}
