//! Core data model: the city-state's treasury, land, grain, populace, buildings,
//! and tax/justice policy, plus the screen modes and the final reckoning.
//!
//! Faithful to George Blank's *Santa Paravia and Fiumaccio* (the C port at
//! `github.com/darkf/paravia`), with two deliberate clean-ups noted in
//! [`super`]: `title_num` is the 0-indexed title ladder (0 = Sir/Lady …
//! 7 = King/Queen, the win), and the bankruptcy floor uses `title_num + 1` so an
//! unranked ruler still gets the original's −10,000-florin cushion.

use serde::{Deserialize, Serialize};

/// The title that wins the game — King / Queen.
pub const WIN_TITLE: i64 = 7;
/// First year of every reign.
pub const START_YEAR: i64 = 1400;

/// The eight noble ranks, indexed by `title_num`.
pub const MALE_TITLES: [&str; 8] = [
    "Sir",
    "Baron",
    "Count",
    "Marquis",
    "Duke",
    "Grand Duke",
    "Prince",
    "King",
];
pub const FEMALE_TITLES: [&str; 8] = [
    "Lady",
    "Baroness",
    "Countess",
    "Marquise",
    "Duchess",
    "Grand Duchess",
    "Princess",
    "Queen",
];

/// The seven historic city-states you may choose to rule.
pub const CITY_NAMES: [&str; 7] = [
    "Santa Paravia",
    "Fiumaccio",
    "Torricella",
    "Molinetto",
    "Fontanile",
    "Romanga",
    "Monterana",
];

/// Whether the ruler takes the male or female title set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

/// The four skill tiers. The divisor scales how much economy each title demands:
/// at Grand Master you need roughly four times the standing of an Apprentice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Apprentice,
    Journeyman,
    Master,
    GrandMaster,
}

impl Difficulty {
    /// The `Total / Difficulty` divisor in the title formula.
    pub fn divisor(self) -> i64 {
        match self {
            Difficulty::Apprentice => 1,
            Difficulty::Journeyman => 2,
            Difficulty::Master => 3,
            Difficulty::GrandMaster => 4,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Apprentice => "Apprentice",
            Difficulty::Journeyman => "Journeyman",
            Difficulty::Master => "Master",
            Difficulty::GrandMaster => "Grand Master",
        }
    }

    pub const ALL: [Difficulty; 4] = [
        Difficulty::Apprentice,
        Difficulty::Journeyman,
        Difficulty::Master,
        Difficulty::GrandMaster,
    ];
}

/// The city-state at one instant, plus the policy and bookkeeping carried across
/// years. Money is integer gold florins; `public_works` and `land_price` are the
/// two genuinely fractional quantities, as in the original.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// The ruler's name (for the hall of fame).
    pub name: String,
    /// The city ruled.
    pub city: String,
    pub gender: Gender,
    pub difficulty: Difficulty,

    /// Current year; the first played year is [`START_YEAR`].
    pub year: i64,
    /// The year the ruler dies of natural causes (a reign of 20–55 years).
    pub year_of_death: i64,

    pub treasury: i64,
    /// Land in hectares.
    pub land: i64,
    /// Grain reserve in steres.
    pub grain: i64,
    /// Grain price per 1000 steres (set each year).
    pub grain_price: i64,
    /// Land price per hectare (set each year).
    pub land_price: f64,
    /// Grain the populace needs this year.
    pub grain_demand: i64,
    /// This year's weather/harvest quality, 0 (drought) to 5 (excellent).
    pub harvest: i64,
    /// Percent of last year's reserve the rats ate.
    pub rats: i64,
    /// Steres of grain this year's fields actually yielded (display).
    pub harvest_yield: i64,

    pub serfs: i64,
    pub soldiers: i64,
    pub merchants: i64,
    pub nobles: i64,
    pub clergy: i64,

    pub marketplaces: i64,
    pub mills: i64,
    pub palace: i64,
    pub cathedral: i64,
    /// Cumulative public-works rating; feeds tax revenue and the title score.
    pub public_works: f64,

    pub customs_duty: i64,
    pub sales_tax: i64,
    pub income_tax: i64,
    /// 1 Very Fair, 2 Moderate, 3 Harsh, 4 Outrageous.
    pub justice: i64,

    /// Current title rank, 0 (Sir/Lady) … 7 (King/Queen).
    pub title_num: i64,
    /// High-water title — rank never drops below what you've earned.
    pub old_title: i64,
}

impl GameState {
    pub fn new(name: String, city: String, gender: Gender, difficulty: Difficulty) -> Self {
        Self {
            name,
            city,
            gender,
            difficulty,
            year: START_YEAR,
            year_of_death: START_YEAR + 40, // overwritten in Game::begin via the RNG
            treasury: 1000,
            land: 10000,
            grain: 5000,
            grain_price: 25,
            land_price: 10.0,
            grain_demand: 0,
            harvest: 3,
            rats: 0,
            harvest_yield: 0,
            serfs: 2000,
            soldiers: 25,
            merchants: 25,
            nobles: 4,
            clergy: 5,
            marketplaces: 0,
            mills: 0,
            palace: 0,
            cathedral: 0,
            public_works: 1.0,
            customs_duty: 25,
            sales_tax: 10,
            income_tax: 5,
            justice: 2,
            title_num: 0,
            old_title: 0,
        }
    }

    /// The current title word (Sir, Baroness, … King, Queen).
    pub fn title(&self) -> &'static str {
        let i = self.title_num.clamp(0, 7) as usize;
        match self.gender {
            Gender::Male => MALE_TITLES[i],
            Gender::Female => FEMALE_TITLES[i],
        }
    }

    /// "Sir Reginald", "Duchess Isabella" — title plus name.
    pub fn full_title(&self) -> String {
        format!("{} {}", self.title(), self.name)
    }

    /// Public-works rating as the integer hundredths the original tallies.
    pub fn public_works_i(&self) -> i64 {
        (self.public_works * 100.0) as i64
    }

    /// Total finished/partial state buildings (for the score).
    pub fn buildings(&self) -> i64 {
        self.marketplaces + self.mills + self.palace + self.cathedral
    }

    /// One-word read on the year's weather, for the report header.
    pub fn weather(&self) -> &'static str {
        match self.harvest {
            0 | 1 => "Drought — famine threatens",
            2 => "Bad weather — poor harvest",
            3 => "Normal weather — average harvest",
            4 => "Good weather — fine harvest",
            _ => "Excellent weather — great harvest!",
        }
    }

    /// The label shown beside the JUSTICE policy.
    pub fn justice_label(&self) -> &'static str {
        match self.justice {
            1 => "Very Fair",
            2 => "Moderate",
            3 => "Harsh",
            _ => "Outrageous",
        }
    }
}

/// Which screen is showing. Saved with the game so a refresh resumes in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Splash,
    /// Naming the ruler, choosing city / gender / difficulty.
    NewGame,
    /// Start-of-year standings, weather, and prices; tap to begin trading.
    YearReport,
    /// Buy and sell grain and land.
    Market,
    /// Distribute grain to the populace for the year.
    Release,
    /// Set the four tax / justice policies.
    Tax,
    /// Invest in buildings and soldiers.
    Purchases,
    /// Showing the head of the pending message queue.
    Interaction,
    GameOver,
}

/// The final reckoning, shown on the game-over screen and posted to the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGame {
    pub won: bool,
    /// Narration of how the reign ended.
    pub cause: String,
    /// Final title word.
    pub title: String,
    pub years: i64,
    pub treasury: i64,
    pub land: i64,
    pub serfs: i64,
    pub score: i64,
    pub rank: String,
    pub recorded: bool,
}
