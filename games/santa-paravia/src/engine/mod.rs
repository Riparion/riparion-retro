//! Pure game engine — no UI or platform dependencies. The UI owns a [`Game`] in
//! a signal and calls these methods; everything here is host-testable.
//!
//! ## A year, as a resumable state machine
//!
//! The original plays one long procedural loop per year. Here that loop is a
//! serialized [`Phase`] cursor walked by [`Game::run_phases`]: automatic phases
//! (harvest & prices, the populace's response, the Baron's raid, banking the
//! taxes, the title check, year's end) run and advance the cursor; the four
//! decision phases (market, grain release, taxes, state purchases) set a [`Mode`]
//! and return. Narration between phases rides the [`Interaction`] queue and is
//! tapped through; when it drains, [`Game::after_queue`] re-enters `run_phases`
//! at the already-advanced cursor, so a refresh resumes mid-year in place.
//!
//! ## Faithfulness & the RNG
//!
//! Formulas track the C port (`github.com/darkf/paravia`). That port's
//! `Random(hi)` is `(int)(rand()/RAND_MAX*hi)` — integer division collapses it to
//! ~0, so the C build is nearly deterministic. That's a porting defect; the
//! design wants a uniform draw. We use [`Game::rnd`] = an **inclusive** `[0, hi]`
//! integer (`rng.ri(hi + 1)`), the original BASIC's intent, atop the workspace's
//! seedable [`GameRng`].

pub mod interaction;
pub use retro_kit::rng;
pub mod scoring;
pub mod state;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use interaction::{Interaction, Response};
use rng::GameRng;
use state::{Difficulty, EndGame, GameState, Gender, Mode, START_YEAR, WIN_TITLE};

/// The Baron Peppone's fixed strength — the lone neighbour who raids a
/// poorly-defended city. He never grows; keep your soldiers ahead of your land.
const BARON_SOLDIERS: i64 = 25;
const BARON_LAND: i64 = 10000;

/// Position within the year — the resume cursor, in play order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Roll the weather, set grain & land prices, then show the year report.
    Harvest,
    /// Buy / sell grain and land (input).
    Market,
    /// Distribute grain to the populace (input).
    Release,
    /// Apply births, deaths, migration, building income, and soldier pay.
    ReleaseResolve,
    /// The Baron raids if your land has outgrown your army.
    Invasion,
    /// Set the four tax / justice policies (input).
    Tax,
    /// Bank the year's revenue; bankrupt rulers are stripped of their estate.
    TaxResolve,
    /// Invest in buildings and soldiers (input).
    Purchases,
    /// Recompute the title; reaching King/Queen wins.
    TitleCheck,
    /// Advance the calendar; the ruler may die of natural causes.
    YearEnd,
}

/// The complete game: world state, UI mode, the year cursor, pending narration,
/// the year's released grain, the raid flag, the outcome, and the RNG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub state: GameState,
    pub mode: Mode,
    pub phase: Phase,
    pub pending: VecDeque<Interaction>,
    /// Grain released this year (carried Release → ReleaseResolve).
    pub released: i64,
    /// Whether the populace's response left the city open to a raid.
    pub invade: bool,
    /// Error to re-show on an input screen after invalid input.
    pub input_error: Option<String>,
    pub outcome: Option<EndGame>,
    pub rng: GameRng,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            state: GameState::new(
                String::new(),
                state::CITY_NAMES[0].to_string(),
                Gender::Male,
                Difficulty::Apprentice,
            ),
            mode: Mode::Splash,
            phase: Phase::Harvest,
            pending: VecDeque::new(),
            released: 0,
            invade: false,
            input_error: None,
            outcome: None,
            rng: GameRng::from_seed(seed),
        }
    }

    /// Begin a new reign. Seeds the state, rolls the lifespan, and runs up to the
    /// first year's report. Idempotent under a double-tap.
    pub fn begin(&mut self, name: String, city: String, gender: Gender, difficulty: Difficulty) {
        self.state = GameState::new(name, city, gender, difficulty);
        // Reign of 20–55 years (original: YearOfDeath = year + 20 + Random(35)).
        self.state.year_of_death = START_YEAR + 20 + self.rnd(35);
        self.pending.clear();
        self.released = 0;
        self.invade = false;
        self.input_error = None;
        self.outcome = None;
        self.phase = Phase::Harvest;
        self.run_phases();
    }

    // ----- RNG idiom -----

    /// `Random(hi)`: a uniform integer in the **inclusive** range `[0, hi]` — the
    /// original BASIC's intent (see the module note on the C port's bug).
    fn rnd(&mut self, hi: i64) -> i64 {
        self.rng.ri(hi + 1)
    }

    // ----- The year cursor -----

    /// Walk phases until one pauses for input, queues narration, or ends the
    /// reign. Re-entered by every submit handler and by [`Self::after_queue`].
    pub(crate) fn run_phases(&mut self) {
        loop {
            if self.outcome.is_some() {
                self.mode = if self.pending.is_empty() {
                    Mode::GameOver
                } else {
                    Mode::Interaction
                };
                return;
            }
            match self.phase {
                Phase::Harvest => {
                    self.generate_harvest();
                    self.new_land_and_grain_prices();
                    self.phase = Phase::Market;
                    self.mode = Mode::YearReport;
                    return;
                }
                Phase::Market => {
                    self.mode = Mode::Market;
                    return;
                }
                Phase::Release => {
                    self.mode = Mode::Release;
                    return;
                }
                Phase::ReleaseResolve => {
                    self.run_release_resolve();
                    self.phase = if self.invade {
                        Phase::Invasion
                    } else {
                        Phase::Tax
                    };
                    if self.show_pending() {
                        return;
                    }
                }
                Phase::Invasion => {
                    self.run_invasion();
                    self.phase = Phase::Tax;
                    if self.show_pending() {
                        return;
                    }
                }
                Phase::Tax => {
                    self.mode = Mode::Tax;
                    return;
                }
                Phase::TaxResolve => {
                    self.run_tax_resolve();
                    self.phase = Phase::Purchases;
                    if self.show_pending() {
                        return;
                    }
                }
                Phase::Purchases => {
                    self.mode = Mode::Purchases;
                    return;
                }
                Phase::TitleCheck => {
                    self.run_title_check();
                    self.phase = Phase::YearEnd;
                    if self.show_pending() {
                        return;
                    }
                }
                Phase::YearEnd => {
                    self.run_year_end();
                    self.phase = Phase::Harvest;
                    if self.show_pending() {
                        return;
                    }
                }
            }
        }
    }

    /// If narration is queued, show it and signal `run_phases` to stop.
    fn show_pending(&mut self) -> bool {
        if self.pending.is_empty() {
            false
        } else {
            self.mode = Mode::Interaction;
            true
        }
    }

    // ----- Interaction queue -----

    fn message(&mut self, text: impl Into<String>) {
        self.pending.push_back(Interaction::Message(text.into()));
    }

    /// Show queued items, or — if none — resume the driver.
    fn advance(&mut self) {
        if self.pending.is_empty() {
            self.after_queue();
        } else {
            self.mode = Mode::Interaction;
        }
    }

    /// The queue drained: resume the driver (which routes to GameOver itself if
    /// the reign has ended).
    fn after_queue(&mut self) {
        self.run_phases();
    }

    /// Acknowledge the head message. Guarded so a double-tap on the last item
    /// can't double-drain the queue and skip a phase.
    pub fn resolve(&mut self, _response: Response) {
        if self.mode != Mode::Interaction {
            return;
        }
        self.pending.pop_front();
        self.advance();
    }

    // ===== Automatic phases =====

    /// Roll the weather and let the rats into the granary.
    fn generate_harvest(&mut self) {
        self.state.harvest = (self.rnd(5) + self.rnd(6)) / 2;
        self.state.rats = self.rnd(50);
        self.state.grain = self.state.grain * (100 - self.state.rats) / 100;
    }

    /// Add this year's field yield to the reserve and set the year's prices, all
    /// driven by harvest quality and the grain supply/demand ratio.
    fn new_land_and_grain_prices(&mut self) {
        let my_random = self.rng.uniform(); // [0, 1)
        let s = &self.state;
        // The land that can actually be worked, capped by the labour force and
        // by twice the grain on hand.
        let mut x = s.land as f64;
        let mut y = (((s.serfs - s.mills) as f64) * 100.0) * 5.0;
        if y < 0.0 {
            y = 0.0;
        }
        if y < x {
            x = y;
        }
        y = s.grain as f64 * 2.0;
        if y < x {
            x = y;
        }
        // Field yield: worked land times the weather (jittered ±0.5).
        y = s.harvest as f64 + (my_random - 0.5);
        let h = (x * y) as i64;
        self.state.grain += h;
        self.state.grain_demand = self.state.nobles * 100
            + self.state.cathedral * 40
            + self.state.merchants * 30
            + self.state.soldiers * 10
            + self.state.serfs * 5;

        let mut land_price = (3.0 * self.state.harvest as f64 + self.rnd(6) as f64 + 10.0) / 10.0;

        let h_abs = h.abs();
        let mut ratio = if h_abs < 1 {
            2.0
        } else {
            (self.state.grain_demand as f64 / h_abs as f64).min(2.0)
        };
        if ratio < 0.8 {
            ratio = 0.8;
        }
        land_price *= ratio;
        if land_price < 1.0 {
            land_price = 1.0;
        }
        self.state.land_price = land_price;
        self.state.grain_price = (((6.0 - self.state.harvest as f64) * 3.0
            + self.rnd(5) as f64
            + self.rnd(5) as f64)
            * 4.0
            * ratio) as i64;
        self.state.harvest_yield = h_abs;
    }

    /// Distribute the released grain and let the populace respond: births,
    /// deaths, migration, and the year's standing income/expense.
    fn run_release_resolve(&mut self) {
        self.state.grain -= self.released;
        let released = self.released;
        let demand = self.state.grain_demand.max(1);

        let before = self.state.serfs;
        if released < self.state.grain_demand - 1 {
            // Shortfall: a few births, heavy deaths scaled by how badly the
            // populace went hungry.
            let shortfall = ((demand - released) as f64 / demand as f64) * 100.0 - 9.0;
            self.serfs_change(3.0, true);
            self.serfs_change(shortfall.max(0.0) + 8.0, false);
        } else {
            // Adequate or generous: strong births, light deaths, and — with low
            // taxes and surplus grain — fresh merchants, nobles, and immigrants.
            self.serfs_change(7.0, true);
            self.serfs_change(3.0, false);

            if self.state.customs_duty + self.state.sales_tax < 35 {
                let m = self.rnd(4);
                self.state.merchants += m;
            }
            if self.state.income_tax < self.rnd(28) {
                let n = self.rnd(2);
                let c = self.rnd(3);
                self.state.nobles += n;
                self.state.clergy += c;
            }
            if released > (self.state.grain_demand as f64 * 1.3) as i64 {
                let per_mille = self.state.serfs as f64 / 1000.0;
                let mut z =
                    (released - self.state.grain_demand) as f64 / self.state.grain_demand as f64
                        * 10.0;
                z *= per_mille * self.rnd(25) as f64;
                z += self.rnd(40) as f64;
                let transplanted = z as i64;
                self.state.serfs += transplanted;
                let new_merchants = (z * self.rng.uniform()).min(50.0) as i64;
                self.state.merchants += new_merchants;
                self.state.nobles += 1;
                self.state.clergy += 2;
                if transplanted > 0 {
                    self.message(format!(
                        "{transplanted} serfs migrate to {}, drawn by your full granaries.",
                        self.state.city
                    ));
                }
            }
        }

        // Net population line (births − deaths from the two serfs_change calls).
        let net = self.state.serfs - before;
        let pop_word = if net >= 0 { "grows" } else { "falls" };
        self.message(format!(
            "The populace {pop_word} to {} souls (was {before}).",
            self.state.serfs
        ));

        // Harsh justice drives serfs away.
        if self.state.justice > 2 {
            let base = self.state.serfs / 100 * (self.state.justice - 2) * (self.state.justice - 2);
            let fleeing = self.rnd(base);
            self.state.serfs -= fleeing;
            if fleeing > 0 {
                self.message(format!("{fleeing} serfs flee your {} justice.", {
                    self.state.justice_label().to_lowercase()
                }));
            }
        }

        // Standing income and the soldiers' wages.
        let market_income = self.state.marketplaces * 75;
        let mill_income = self.state.mills * (55 + self.rnd(250));
        let soldier_pay = self.state.soldiers * 3;
        self.state.treasury += market_income + mill_income - soldier_pay;
        let mut lines = Vec::new();
        if market_income > 0 {
            lines.push(format!("markets +{market_income}"));
        }
        if mill_income > 0 {
            lines.push(format!("mills +{mill_income}"));
        }
        lines.push(format!("soldiers −{soldier_pay}"));
        self.message(format!("Ledger: {} florins.", lines.join(", ")));

        // A poorly-defended, sprawling city invites the Baron.
        self.invade =
            (self.state.land / 1000 > self.state.soldiers) || (self.state.land / 500 > self.state.soldiers);
    }

    /// One births-or-deaths roll. `scale` blends an integer band with its
    /// fractional part: `delta = ((Random(floor scale) + frac) * serfs) / 100`.
    fn serfs_change(&mut self, scale: f64, birth: bool) {
        let band = scale.floor() as i64;
        let frac = scale - band as f64;
        let delta = (((self.rnd(band) as f64 + frac) * self.state.serfs as f64) / 100.0) as i64;
        if birth {
            self.state.serfs += delta;
        } else {
            self.state.serfs -= delta;
        }
    }

    /// The Baron Peppone seizes land and bloodies your garrison.
    fn run_invasion(&mut self) {
        let mut land_taken = BARON_SOLDIERS * 1000 - BARON_LAND / 3;
        if land_taken > self.state.land - 5000 {
            land_taken = (self.state.land - 5000) / 2;
        }
        self.state.land -= land_taken;
        let mut dead = self.rnd(40);
        if dead > self.state.soldiers - 15 {
            dead = self.state.soldiers - 15;
        }
        self.state.soldiers -= dead;
        self.message(format!(
            "Baron Peppone invades {} and seizes {land_taken} hectares!",
            self.state.city
        ));
        self.message(format!("Your garrison loses {dead} soldiers in the battle."));
    }

    /// The four tax revenue streams at the current policy. Pure — used both for
    /// the live preview on the tax screen and for banking in [`Self::run_tax_resolve`].
    /// Returns `(customs, sales, income, justice, total)`.
    pub fn projected_revenue(&self) -> (i64, i64, i64, i64, i64) {
        let s = &self.state;
        let pw = s.public_works;
        let mut y = 150.0 - s.sales_tax as f64 - s.customs_duty as f64 - s.income_tax as f64;
        if y < 1.0 {
            y = 1.0;
        }
        y /= 100.0;

        let justice = (s.justice * 300 - 500) * s.title_num;

        let mut customs = (s.nobles * 180) as f64 + (s.clergy * 75) as f64 + (s.merchants * 20) as f64 * y;
        customs += (pw * 100.0).floor();
        let customs = (s.customs_duty as f64 / 100.0 * customs) as i64;

        let mut sales = s.nobles * 50 + s.merchants * 25 + (pw * 10.0) as i64;
        sales = (sales as f64 * (y * (5 - s.justice) as f64 * s.sales_tax as f64)) as i64;
        sales /= 200;

        let mut income = s.nobles * 250 + (pw * 20.0) as i64;
        income += (10.0 * s.justice as f64 * s.nobles as f64 * y) as i64;
        income *= s.income_tax;
        income /= 100;

        let total = customs + sales + income + justice;
        (customs, sales, income, justice, total)
    }

    /// Total revenue banked this year at the current policy — the figure
    /// [`Self::run_tax_resolve`] adds to the treasury.
    pub fn total_revenue(&self) -> i64 {
        self.projected_revenue().4
    }

    /// Bank the year's taxes, deepen any deficit, and seize the estate of a
    /// ruler too deep in debt.
    fn run_tax_resolve(&mut self) {
        self.state.treasury += self.total_revenue();
        if self.state.treasury < 0 {
            self.state.treasury = (self.state.treasury as f64 * 1.5) as i64;
        }
        if self.state.treasury < -10000 * (self.state.title_num + 1) {
            // Bankrupt — creditors strip the estate (original SeizeAssets).
            self.state.marketplaces = 0;
            self.state.palace = 0;
            self.state.cathedral = 0;
            self.state.mills = 0;
            self.state.land = 6000;
            self.state.public_works = 1.0;
            self.state.treasury = 100;
            self.message(format!(
                "{} is bankrupt! Creditors seize your buildings and lands.",
                self.state.full_title()
            ));
        }
    }

    /// Recompute the title from twelve capped measures of the city's standing.
    /// Reaching King/Queen wins the game.
    fn run_title_check(&mut self) {
        let s = &self.state;
        let total = limit10(s.marketplaces, 1)
            + limit10(s.palace, 1)
            + limit10(s.cathedral, 1)
            + limit10(s.mills, 1)
            + limit10(s.treasury, 5000)
            + limit10(s.land, 6000)
            + limit10(s.merchants, 50)
            + limit10(s.nobles, 5)
            + limit10(s.soldiers, 50)
            + limit10(s.clergy, 10)
            + limit10(s.serfs, 2000)
            + limit10(s.public_works_i(), 500);
        let new_title = (total / s.difficulty.divisor() - s.justice).clamp(0, 7);

        if new_title > self.state.old_title {
            self.state.old_title = new_title;
            self.state.title_num = new_title;
            if new_title == WIN_TITLE {
                let cause = format!(
                    "After {} years, {} of {} is crowned — long live the {}!",
                    self.state.year - START_YEAR,
                    self.state.name,
                    self.state.city,
                    self.state.title()
                );
                self.outcome = Some(scoring::build_end(&self.state, true, cause));
                self.message(format!(
                    "The crown is yours! {} of {} ascends the throne.",
                    self.state.full_title(),
                    self.state.city
                ));
            } else {
                self.message(format!(
                    "Your good rule is rewarded — {} is now {}!",
                    self.state.name,
                    self.state.title()
                ));
            }
        } else {
            self.state.title_num = self.state.old_title;
        }
    }

    /// Advance the calendar; the ruler may die of natural causes.
    fn run_year_end(&mut self) {
        self.state.year += 1;
        if self.state.year >= self.state.year_of_death {
            let reason = self.death_reason();
            let cause = format!("{} has died {reason}", self.state.full_title());
            self.outcome = Some(scoring::build_end(&self.state, false, cause.clone()));
            self.message(format!("Sad news — {cause}"));
        }
    }

    /// The cause of a natural death, period-appropriate.
    fn death_reason(&mut self) -> &'static str {
        if self.state.year > 1450 {
            return "of old age, after a long reign.";
        }
        match self.rnd(8) {
            0..=3 => "of pneumonia, after a cold winter in a draughty castle.",
            4 => "of typhoid, from contaminated water.",
            5 => "in a smallpox epidemic.",
            6 => "at the hands of robbers while travelling.",
            _ => "of food poisoning.",
        }
    }

    // ===== Decision-phase inputs =====

    /// Tap-through on the start-of-year report → open the market.
    pub fn advance_from_report(&mut self) {
        if self.mode != Mode::YearReport {
            return;
        }
        self.run_phases();
    }

    /// Buy `amount` steres of grain at this year's price.
    pub fn buy_grain(&mut self, amount: i64) -> Result<(), String> {
        if self.mode != Mode::Market {
            return Ok(());
        }
        let amount = amount.max(0);
        let cost = amount * self.state.grain_price / 1000;
        if cost > self.state.treasury {
            return self.input_err("You can't afford that much grain.");
        }
        self.state.treasury -= cost;
        self.state.grain += amount;
        self.input_error = None;
        Ok(())
    }

    /// Sell `amount` steres of grain at this year's price.
    pub fn sell_grain(&mut self, amount: i64) -> Result<(), String> {
        if self.mode != Mode::Market {
            return Ok(());
        }
        let amount = amount.max(0);
        if amount > self.state.grain {
            return self.input_err("You don't have that much grain.");
        }
        self.state.treasury += amount * self.state.grain_price / 1000;
        self.state.grain -= amount;
        self.input_error = None;
        Ok(())
    }

    /// Buy `amount` hectares of land at this year's price.
    pub fn buy_land(&mut self, amount: i64) -> Result<(), String> {
        if self.mode != Mode::Market {
            return Ok(());
        }
        let amount = amount.max(0);
        let cost = (amount as f64 * self.state.land_price) as i64;
        if cost > self.state.treasury {
            return self.input_err("You can't afford that much land.");
        }
        self.state.land += amount;
        self.state.treasury -= cost;
        self.input_error = None;
        Ok(())
    }

    /// Sell `amount` hectares of land — you must keep at least 5,000.
    pub fn sell_land(&mut self, amount: i64) -> Result<(), String> {
        if self.mode != Mode::Market {
            return Ok(());
        }
        let amount = amount.max(0);
        if amount > self.state.land - 5000 {
            return self.input_err("You must keep at least 5,000 hectares.");
        }
        self.state.land -= amount;
        self.state.treasury += (amount as f64 * self.state.land_price) as i64;
        self.input_error = None;
        Ok(())
    }

    /// Leave the market and move on to feeding the populace.
    pub fn done_market(&mut self) {
        if self.mode != Mode::Market {
            return;
        }
        self.input_error = None;
        self.phase = Phase::Release;
        self.advance();
    }

    /// Grain you must release at minimum (20% of the reserve).
    pub fn release_min(&self) -> i64 {
        self.state.grain / 5
    }

    /// Grain you may release at most (keeping 20% in reserve).
    pub fn release_max(&self) -> i64 {
        self.state.grain - self.release_min()
    }

    /// Release `amount` steres of grain for the year's consumption.
    pub fn submit_release(&mut self, amount: i64) -> Result<(), String> {
        if self.mode != Mode::Release {
            return Ok(());
        }
        let amount = amount.max(0);
        if amount < self.release_min() {
            return self.input_err("You must release at least 20% of your reserve.");
        }
        if amount > self.release_max() {
            return self.input_err("You must keep at least 20% of your reserve.");
        }
        self.released = amount;
        self.input_error = None;
        self.phase = Phase::ReleaseResolve;
        self.advance();
        Ok(())
    }

    pub fn set_customs_duty(&mut self, v: i64) {
        if self.mode == Mode::Tax {
            self.state.customs_duty = v.clamp(0, 100);
        }
    }
    pub fn set_sales_tax(&mut self, v: i64) {
        if self.mode == Mode::Tax {
            self.state.sales_tax = v.clamp(0, 50);
        }
    }
    pub fn set_income_tax(&mut self, v: i64) {
        if self.mode == Mode::Tax {
            self.state.income_tax = v.clamp(0, 25);
        }
    }
    pub fn set_justice(&mut self, v: i64) {
        if self.mode == Mode::Tax {
            self.state.justice = v.clamp(1, 4);
        }
    }

    /// Confirm this year's policy and bank the resulting revenue.
    pub fn done_tax(&mut self) {
        if self.mode != Mode::Tax {
            return;
        }
        self.phase = Phase::TaxResolve;
        self.advance();
    }

    /// Buy one marketplace (1,000 fl): +5 merchants, +1.0 public works.
    pub fn buy_marketplace(&mut self) {
        if self.mode == Mode::Purchases && self.state.treasury >= 1000 {
            self.state.marketplaces += 1;
            self.state.merchants += 5;
            self.state.treasury -= 1000;
            self.state.public_works += 1.0;
        }
    }
    /// Buy one woolen mill (2,000 fl): +0.25 public works.
    pub fn buy_mill(&mut self) {
        if self.mode == Mode::Purchases && self.state.treasury >= 2000 {
            self.state.mills += 1;
            self.state.treasury -= 2000;
            self.state.public_works += 0.25;
        }
    }
    /// Buy a piece of a palace (3,000 fl): +0–2 nobles, +0.5 public works.
    pub fn buy_palace(&mut self) {
        if self.mode == Mode::Purchases && self.state.treasury >= 3000 {
            self.state.palace += 1;
            let n = self.rnd(2);
            self.state.nobles += n;
            self.state.treasury -= 3000;
            self.state.public_works += 0.5;
        }
    }
    /// Buy a piece of a cathedral (5,000 fl): +0–6 clergy, +1.0 public works.
    pub fn buy_cathedral(&mut self) {
        if self.mode == Mode::Purchases && self.state.treasury >= 5000 {
            self.state.cathedral += 1;
            let c = self.rnd(6);
            self.state.clergy += c;
            self.state.treasury -= 5000;
            self.state.public_works += 1.0;
        }
    }
    /// Equip a platoon (500 fl): +20 soldiers from 20 serfs.
    pub fn buy_soldiers(&mut self) {
        if self.mode == Mode::Purchases && self.state.treasury >= 500 && self.state.serfs >= 20 {
            self.state.soldiers += 20;
            self.state.serfs -= 20;
            self.state.treasury -= 500;
        }
    }

    /// Close the books on the year → title check, then the new year.
    pub fn done_purchases(&mut self) {
        if self.mode != Mode::Purchases {
            return;
        }
        self.phase = Phase::TitleCheck;
        self.advance();
    }

    fn input_err(&mut self, msg: &str) -> Result<(), String> {
        self.input_error = Some(msg.to_string());
        Err(msg.to_string())
    }
}

/// `limit10(n, d) = n / d`, capped at 10 (the original caps the *upside* only —
/// a negative measure such as a debt-ridden treasury can still pull the total
/// down).
fn limit10(num: i64, denom: i64) -> i64 {
    (num / denom).min(10)
}

#[cfg(test)]
mod tests;
