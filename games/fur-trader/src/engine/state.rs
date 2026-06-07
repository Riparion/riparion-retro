//! Core state: the expedition's ledger. BASIC variable names from the 1976
//! listing appear in the doc comments for auditability against the spec.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Splash,
    /// Distributing this year's 190 furs among the four kinds of pelts.
    Allocate,
    /// Picking one of the three forts.
    FortSelect,
    /// "Do you want to trade at another fort?" — change your mind or set out.
    FortConfirm,
    /// The expedition journal: travel, events, the sale, next-year/retire.
    Results,
    GameOver,
}

/// The four kinds of pelts, in the sale-formula order F(1)..F(4)
/// (line 1418: `I=M1*F(1)+B1*F(2)+E1*F(3)+D1*F(4)+I`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fur {
    Mink,
    Beaver,
    Ermine,
    Fox,
}

pub const FUR_ORDER: [Fur; 4] = [Fur::Mink, Fur::Beaver, Fur::Ermine, Fur::Fox];

impl Fur {
    pub fn name(&self) -> &'static str {
        match self {
            Fur::Mink => "MINK",
            Fur::Beaver => "BEAVER",
            Fur::Ermine => "ERMINE",
            Fur::Fox => "FOX",
        }
    }
}

/// The three forts (line 1110: "ANSWER 1, 2, OR 3").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fort {
    /// Fort 1: Fort Hochelaga (Montreal).
    Hochelaga,
    /// Fort 2: Fort Stadacona (Quebec).
    Stadacona,
    /// Fort 3: Fort New York.
    NewYork,
}

pub const FORT_ORDER: [Fort; 3] = [Fort::Hochelaga, Fort::Stadacona, Fort::NewYork];

/// Everything the UI needs to render a fort card, and the engine needs to
/// charge for the trip — one source of truth (lines 1102–1150, 1180/1244/1320).
pub struct FortInfo {
    pub name: &'static str,
    pub city: &'static str,
    pub supplies: f64,
    pub travel: f64,
    pub flavor: &'static str,
    pub risk: &'static str,
}

impl FortInfo {
    pub fn cost(&self) -> f64 {
        self.supplies + self.travel
    }
}

pub fn fort_info(fort: Fort) -> FortInfo {
    match fort {
        Fort::Hochelaga => FortInfo {
            name: "FORT HOCHELAGA",
            city: "Montreal",
            supplies: 150.0,
            travel: 10.0,
            flavor: "Under the protection of the French army. The easiest route — \
                     but the fort is far from any seaport, so the value you receive \
                     for your furs will be low and the cost of supplies higher than \
                     at Forts Stadacona or New York.",
            risk: "No danger",
        },
        Fort::Stadacona => FortInfo {
            name: "FORT STADACONA",
            city: "Quebec",
            supplies: 125.0,
            travel: 15.0,
            flavor: "Under the protection of the French army, but you must make a \
                     portage and cross the Lachine Rapids. A hard route — harder \
                     than Hochelaga but easier than New York. You will receive an \
                     average value for your furs at an average cost of supplies.",
            risk: "Portage & rapids",
        },
        Fort::NewYork => FortInfo {
            name: "FORT NEW YORK",
            city: "New York",
            supplies: 80.0,
            travel: 25.0,
            flavor: "Under Dutch control — you must cross through Iroquois land. \
                     The most difficult route, but you will receive the highest \
                     value for your furs and the cost of your supplies will be \
                     lower than at all the other forts.",
            risk: "Iroquois land",
        },
    }
}

/// One line of the expedition journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogLine {
    pub text: String,
    /// Calamities (rapids, raids, the ambush) get the danger tint.
    pub alert: bool,
    /// Opens a new expedition entry — the UI puts a gap above it.
    pub head: bool,
}

impl LogLine {
    pub fn line(text: impl Into<String>) -> Self {
        Self { text: text.into(), alert: false, head: false }
    }

    pub fn alert(text: impl Into<String>) -> Self {
        Self { text: text.into(), alert: true, head: false }
    }

    pub fn head(text: impl Into<String>) -> Self {
        Self { text: text.into(), alert: false, head: true }
    }
}

/// The trading company's books. (Original BASIC variables in parens.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// Expedition year, 1-based. Open-ended: ends by death, bankruptcy,
    /// or retiring.
    pub year: i64,
    /// Savings in dollars (I). Starts at $600 (line 16).
    pub savings: f64,
    /// Pelt counts indexed by `Fur` (F(1)..F(4)).
    pub furs: [i64; 4],
    /// Which fur the allocation prompt is on (0..=4; 4 = all answered).
    pub cursor: usize,
    /// Start-of-year ermine price (E1, line 261) — Fort New York sells
    /// ermine at this carried-over high price.
    pub start_ermine_price: f64,
    /// Start-of-year beaver price (B1, line 262) — likewise at New York.
    pub start_beaver_price: f64,
    /// Tentative fort pick, committed by `confirm_fort`.
    pub fort: Option<Fort>,
    /// Prices the committed sale used, indexed by `Fur` (for the ledger).
    pub sale_prices: [f64; 4],
    /// Proceeds per fur from the committed sale, for the results ledger.
    pub last_sale: Option<[f64; 4]>,
}

pub const STARTING_SAVINGS: f64 = 600.0;
pub const FURS_PER_YEAR: i64 = 190;
/// The cheapest expedition (Fort New York, $80 + $25): below this at the
/// start of a year the company is bankrupt.
pub const MIN_EXPEDITION_COST: f64 = 105.0;

impl State {
    pub fn initial() -> Self {
        Self {
            year: 0,
            savings: STARTING_SAVINGS,
            furs: [0; 4],
            cursor: 0,
            start_ermine_price: 1.0,
            start_beaver_price: 1.0,
            fort: None,
            sale_prices: [0.0; 4],
            last_sale: None,
        }
    }

    pub fn allocated(&self) -> i64 {
        self.furs.iter().sum()
    }
}
