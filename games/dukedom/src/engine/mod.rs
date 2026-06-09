//! Pure game engine — no UI or platform dependencies. The UI owns a [`Game`] in
//! a signal and calls these methods; everything here is host-testable.
//!
//! ## A year, as a resumable state machine
//!
//! Dukedom's original is one long procedural loop per year. Here that loop is a
//! serialized [`Phase`] cursor walked by [`Game::run_phases`]: automatic phases
//! run and advance the cursor; phases that need the player set a [`Mode`] and
//! return. Full-screen decisions (feed / land / plant) are their own modes;
//! in-stride decisions that arise *inside* an automatic phase — the King's
//! demands, the chaos of war — ride the [`Interaction`] queue and are answered
//! through [`Game::resolve`]. When the queue drains, [`Game::after_queue`]
//! re-enters `run_phases` at the (already advanced) cursor, so a refresh resumes
//! mid-year exactly in place — including mid-war (the [`WarCtx`] is serialized).

pub mod interaction;
pub use retro_kit::rng;
pub mod scoring;
pub mod state;

mod phases;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use interaction::{Interaction, Response};
use rng::GameRng;
use state::{EndGame, GameState, Mode, YearReport, F3, SEED_G, SEED_L, SEED_P, SEED_S};

/// Position within the year — the resume cursor. Order matches the original
/// main loop (`imports/dukedom.c` lines 1079-1121).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Roll the year over: snapshot the report, increment the year, reset ledgers.
    LastYearsResults,
    /// Lose/win gate, then the King's possible double-tax demand.
    EndOfGameCheck,
    /// Allocate grain to feed the peasants (input).
    FeedPeasants,
    /// Apply starvation and the unrest it stirs.
    Starvation,
    /// Buy or sell land (input).
    LandAction,
    /// Resolve the King's war, if you provoked it.
    WarWithKing,
    /// Choose acres to plant (input).
    Plant,
    /// Degrade cropped soil; let fallow land recover.
    UpdateLandTables,
    /// Compute the harvest yield, rat losses, and the King's peasant levy.
    CropYield,
    /// A rival Duke may threaten war (input mid-phase).
    War,
    /// Plague, births, and natural deaths.
    Population,
    /// Bank the harvest, pay the castle and the royal tax.
    Harvest,
    /// Decay cumulative unrest, then roll to the next year.
    UpdateUnrest,
}

/// Whether an automatic phase finished or paused for a queued decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    Pause,
}

/// Transient state of an active war with a rival Duke, carried across the
/// attack-first and mercenary-hire prompts (the original's `X2`/`X4`/`X1`/`X3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarCtx {
    /// Enemy army strength (X2), updated by a failed first strike.
    pub enemy_strength: i64,
    /// Your troop effectiveness multiplier (X4): 1 when calm, 0 once unrest ≥ 16.
    pub troop_mult: i64,
    /// Your defensive threshold (X1), used to size first-strike casualties.
    pub threshold: i64,
    /// The war-roll value (X3), also used in first-strike casualties.
    pub defense_x3: i64,
}

/// The complete game: world state, UI mode, the year cursor, pending
/// messages/decisions, an active war (if any), and the RNG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub state: GameState,
    pub mode: Mode,
    pub phase: Phase,
    pub pending: VecDeque<Interaction>,
    /// An in-progress war with a rival Duke, awaiting your decisions.
    pub war: Option<WarCtx>,
    /// Grain allocated to food this year (carried feed → starvation).
    pub fed_grain: i64,
    /// Land buy price quoted this year (computed once, RNG-derived). The sell
    /// price is always one less — see [`Game::land_sell_price`].
    pub land_buy_price: i64,
    /// Error to re-show on an input screen after invalid input.
    pub input_error: Option<String>,
    /// Standings entering the current year, for the report screen.
    pub last_report: Option<YearReport>,
    pub outcome: Option<EndGame>,
    pub rng: GameRng,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            state: GameState::new(String::new()),
            mode: Mode::Splash,
            phase: Phase::LastYearsResults,
            pending: VecDeque::new(),
            war: None,
            fed_grain: 0,
            land_buy_price: 0,
            input_error: None,
            last_report: None,
            outcome: None,
            rng: GameRng::from_seed(seed),
        }
    }

    /// Begin a new reign under `duke`: seed state, roll the per-game event means,
    /// and run up to the first year's report. Idempotent under a double-tap.
    pub fn begin(&mut self, duke: String) {
        self.state = GameState::new(duke);
        for (i, v) in SEED_P.iter().enumerate() {
            self.state.p[i + 1] = *v;
        }
        for (i, v) in SEED_L.iter().enumerate() {
            self.state.l[i + 1] = *v;
        }
        for (i, v) in SEED_G.iter().enumerate() {
            self.state.g[i + 1] = *v;
        }
        for (i, v) in SEED_S.iter().enumerate() {
            self.state.s[i + 1] = *v;
        }
        // Per-game event means R[1..8] (BASIC start_new_game, lines 102-117).
        self.state.r[1] = self.gauss(4, 7);
        self.state.r[2] = self.gauss(4, 8);
        self.state.r[3] = self.gauss(4, 6);
        self.state.r[4] = self.gauss(3, 8);
        self.state.r[5] = self.gauss(5, 8);
        self.state.r[6] = self.gauss(3, 6);
        self.state.r[7] = self.gauss(3, 8);
        self.state.r[8] = self.gauss(4, 8);

        self.pending.clear();
        self.war = None;
        self.fed_grain = 0;
        self.land_buy_price = 0;
        self.input_error = None;
        self.last_report = None;
        self.outcome = None;
        self.phase = Phase::LastYearsResults;
        self.run_phases();
    }

    // ----- BASIC-faithful RNG idioms (imports/dukedom.c lines 122-174) -----

    /// `rnd(1)`: a float in [0.000, 0.999], quantized in thousandths like the C
    /// `(rand() % 1000) / 1000.0`.
    fn rnd1(&mut self) -> f64 {
        self.rng.ri(1000) as f64 / 1000.0
    }

    /// `FNR(Q1,Q2) = rnd(1)*(1+Q2-Q1)+Q1`, truncated toward zero.
    fn fnr(&mut self, q1: i64, q2: i64) -> i64 {
        (self.rnd1() * (1 + q2 - q1) as f64 + q1 as f64) as i64
    }

    /// `FNX(k) = FNR(-F3,F3) + R[k]`: a small spread around the per-game mean R[k].
    fn fnx(&mut self, k: usize) -> i64 {
        self.fnr(-F3, F3) + self.state.r[k]
    }

    /// `gauss(Q1,Q2)`: midrange-skewed draw used to seed the R[] means.
    fn gauss(&mut self, q1: i64, q2: i64) -> i64 {
        let q3 = self.fnr(q1, q2);
        if self.fnr(q1, q2) > 5 {
            let extra = self.fnr(q1, q2);
            (q3 + extra) / 2
        } else {
            q3
        }
    }

    // ----- The year cursor -----

    /// Roll the year over (BASIC `last_years_results`, minus the printing — the
    /// report renders from [`Self::last_report`]).
    fn begin_year(&mut self) {
        self.state.n_y += 1;
        self.last_report = Some(YearReport {
            year: self.state.n_y,
            peasants: self.state.n_p,
            land: self.state.n_l,
            grain: self.state.n_g,
            tiers: self.state.s,
            p: self.state.p,
            l: self.state.l,
            g: self.state.g,
        });
        self.state.p = [0; 9];
        self.state.l = [0; 4];
        self.state.g = [0; 11];
        self.state.p[1] = self.state.n_p;
        self.state.l[1] = self.state.n_l;
        self.state.g[1] = self.state.n_g;
        self.fed_grain = 0;
    }

    /// Walk phases until one pauses for input, queues a message/decision, or ends
    /// the reign. Re-entered by every submit handler and by [`Self::after_queue`].
    pub(crate) fn run_phases(&mut self) {
        loop {
            if self.outcome.is_some() {
                self.mode = Mode::GameOver;
                return;
            }
            match self.phase {
                Phase::LastYearsResults => {
                    self.begin_year();
                    self.phase = Phase::EndOfGameCheck;
                    self.mode = Mode::YearReport;
                    return;
                }
                Phase::EndOfGameCheck => {
                    if let Some(end) = self.check_end_of_game() {
                        self.set_outcome(end);
                        self.mode = Mode::GameOver;
                        return;
                    }
                    self.state.u1 = 0;
                    self.phase = Phase::FeedPeasants;
                    if self.state.k > 0 {
                        self.pending.push_back(Interaction::DoubleTax);
                        self.mode = Mode::Interaction;
                        return;
                    }
                }
                Phase::FeedPeasants => {
                    self.mode = Mode::Feed;
                    return;
                }
                Phase::Starvation => {
                    self.run_starvation();
                    if self.settle_auto(Phase::LandAction) {
                        return;
                    }
                }
                Phase::LandAction => {
                    self.prepare_land_prices();
                    self.mode = Mode::Land;
                    return;
                }
                Phase::WarWithKing => {
                    if self.state.k == -2 {
                        self.run_war_with_king();
                    }
                    if self.settle_auto(Phase::Plant) {
                        return;
                    }
                }
                Phase::Plant => {
                    self.mode = Mode::Plant;
                    return;
                }
                Phase::UpdateLandTables => {
                    self.run_update_land_tables();
                    self.phase = Phase::CropYield;
                }
                Phase::CropYield => {
                    let flow = self.run_crop_yield();
                    self.phase = Phase::War;
                    match flow {
                        Flow::Pause => {
                            self.mode = Mode::Interaction;
                            return;
                        }
                        Flow::Continue => {
                            if self.show_pending() {
                                return;
                            }
                        }
                    }
                }
                Phase::War => {
                    // run_war advances the cursor to Population itself; the war's
                    // sub-decisions resolve through the queue, not by re-entry.
                    let flow = self.run_war();
                    if self.pause_or_end() {
                        return;
                    }
                    match flow {
                        Flow::Pause => {
                            self.mode = Mode::Interaction;
                            return;
                        }
                        Flow::Continue => {
                            if self.show_pending() {
                                return;
                            }
                        }
                    }
                }
                Phase::Population => {
                    self.run_population();
                    if self.settle_auto(Phase::Harvest) {
                        return;
                    }
                }
                Phase::Harvest => {
                    self.run_harvest();
                    if self.settle_auto(Phase::UpdateUnrest) {
                        return;
                    }
                }
                Phase::UpdateUnrest => {
                    self.run_update_unrest();
                    self.phase = Phase::LastYearsResults;
                }
            }
        }
    }

    /// If the reign has ended, route to the game-over screen (showing any final
    /// messages first). Returns whether `run_phases` should stop.
    fn pause_or_end(&mut self) -> bool {
        if self.outcome.is_some() {
            self.mode = if self.pending.is_empty() {
                Mode::GameOver
            } else {
                Mode::Interaction
            };
            return true;
        }
        false
    }

    /// If messages are queued, show them and signal `run_phases` to stop.
    fn show_pending(&mut self) -> bool {
        if self.pending.is_empty() {
            false
        } else {
            self.mode = Mode::Interaction;
            true
        }
    }

    /// Settle a finished automatic phase: end the reign if it ended, otherwise
    /// advance the cursor to `next` and pause if it queued any messages. Returns
    /// whether `run_phases` should stop.
    fn settle_auto(&mut self, next: Phase) -> bool {
        if self.pause_or_end() {
            return true;
        }
        self.phase = next;
        self.show_pending()
    }

    // ----- Interaction queue -----

    pub(crate) fn message(&mut self, text: impl Into<String>) {
        self.pending.push_back(Interaction::Message(text.into()));
    }

    /// Show queued items, or — if none — resume the driver.
    pub(crate) fn advance(&mut self) {
        if self.pending.is_empty() {
            self.after_queue();
        } else {
            self.mode = Mode::Interaction;
        }
    }

    /// The queue drained: resume the driver (which routes to GameOver itself if
    /// the reign has ended).
    pub(crate) fn after_queue(&mut self) {
        self.run_phases();
    }

    /// Answer the head interaction. Guarded so a double-tap on the last item
    /// can't double-drain the queue.
    pub fn resolve(&mut self, response: Response) {
        if self.mode != Mode::Interaction {
            return;
        }
        let Some(head) = self.pending.front().cloned() else {
            self.advance();
            return;
        };
        // Each decision answers only to the response shape it expects; an
        // unexpected shape is ignored (the prompt stays up) rather than silently
        // resolved with a default.
        match head {
            Interaction::Message(_) => {
                self.pending.pop_front();
                self.advance();
            }
            Interaction::DoubleTax => {
                let Some(pay) = as_bool(response) else { return };
                self.pending.pop_front();
                // Pay (Yes) → double tax this year (K=2); refuse (No) → war path (K=-1).
                self.state.k = if pay { 2 } else { -1 };
                self.advance();
            }
            Interaction::KingLevy { peasants, grain } => {
                let Some(supply) = as_bool(response) else { return };
                self.pending.pop_front();
                self.apply_king_levy(peasants, grain, supply);
                self.advance();
            }
            Interaction::WarAttack => {
                let Some(attack) = as_bool(response) else { return };
                self.pending.pop_front();
                self.resolve_attack_choice(attack);
                self.advance();
            }
            Interaction::WarMercenary { .. } => {
                let Response::Amount(n) = response else { return };
                self.pending.pop_front();
                self.resolve_mercenaries(n);
                self.advance();
            }
        }
    }

    // ----- Full-screen inputs -----

    /// Tap-through on the start-of-year report.
    pub fn advance_from_report(&mut self) {
        if self.mode != Mode::YearReport {
            return;
        }
        self.run_phases();
    }

    /// Allocate `v` HL of grain to feed the peasants this winter.
    pub fn submit_feed(&mut self, v: i64) -> Result<(), String> {
        if self.mode != Mode::Feed {
            return Ok(());
        }
        let v = v.max(0);
        if v > self.state.n_g {
            return self.input_err("You don't have that much grain.");
        }
        if v / self.state.n_p.max(1) < 11 && v != self.state.n_g {
            self.state.u1 += 3;
            return self
                .input_err("The peasants demonstrate with sharpened scythes — feed them more!");
        }
        self.state.g[2] = -v;
        self.state.n_g += self.state.g[2];
        self.fed_grain = v;
        self.input_error = None;
        self.phase = Phase::Starvation;
        self.advance_after_input();
        Ok(())
    }

    /// This year's land sale price — always one HL below the buy price.
    pub fn land_sell_price(&self) -> i64 {
        self.land_buy_price - 1
    }

    /// Buy `v` HA of land at this year's quoted price.
    pub fn submit_buy_land(&mut self, v: i64) -> Result<(), String> {
        if self.mode != Mode::Land {
            return Ok(());
        }
        let v = v.max(0);
        let cost = v.saturating_mul(self.land_buy_price);
        if cost > self.state.n_g {
            return self.input_err("You don't have enough grain for that much land.");
        }
        self.state.g[3] = -cost;
        self.state.l[2] = v;
        self.state.s[3] += v;
        if v > 0 {
            self.state.n_l += v;
            self.state.n_g += self.state.g[3];
        }
        self.finish_land();
        Ok(())
    }

    /// Sell `v` HA of land at this year's quoted price.
    pub fn submit_sell_land(&mut self, v: i64) -> Result<(), String> {
        if self.mode != Mode::Land {
            return Ok(());
        }
        let v = v.max(0);
        let good = self.state.good_land();
        if v > good {
            return self.input_err(&format!("You only have {good} HA of good land to sell."));
        }
        let price = self.land_sell_price();
        let mut grain = v.saturating_mul(price);
        if grain > 4000 {
            return self.input_err("No buyers have that much grain — sell less.");
        }
        self.state.l[2] = -v;
        // Remove acres worst-good-land-first (tiers 3 → 1), matching the BASIC.
        let mut remaining = v;
        for j1 in (1..=3).rev() {
            if remaining <= self.state.s[j1] {
                self.state.s[j1] -= remaining;
                break;
            }
            remaining -= self.state.s[j1];
            self.state.s[j1] = 0;
        }
        self.state.n_l += self.state.l[2];
        if self.state.n_l < 10 {
            if let Some(end) = self.check_end_of_game() {
                self.set_outcome(end);
                self.finish_land();
                return Ok(());
            }
        }
        if price < 4 {
            grain /= 2;
            self.message(
                "The High King appropriates half your earnings for selling at such a low price.",
            );
        }
        self.state.g[3] = grain;
        self.state.n_g += grain;
        self.finish_land();
        Ok(())
    }

    /// Plant `v` HA of land (2 HL of seed grain per HA).
    pub fn submit_plant(&mut self, v: i64) -> Result<(), String> {
        if self.mode != Mode::Plant {
            return Ok(());
        }
        let v = v.max(0);
        if v > self.state.n_l {
            return self.input_err(&format!("You only have {} HA of land.", self.state.n_l));
        }
        if v > 4 * self.state.n_p {
            return self.input_err(&format!(
                "Your peasants can only plant {} HA.",
                4 * self.state.n_p
            ));
        }
        let seed = v.saturating_mul(2);
        if seed > self.state.n_g {
            return self.input_err("You don't have enough grain for seed (2 HL per HA).");
        }
        self.state.g[4] = -seed;
        self.state.n_g += self.state.g[4];
        self.state.g[8] = v;
        self.input_error = None;
        self.phase = Phase::UpdateLandTables;
        self.advance_after_input();
        Ok(())
    }

    fn finish_land(&mut self) {
        self.input_error = None;
        self.phase = Phase::WarWithKing;
        self.advance_after_input();
    }

    /// After an input applies: show any queued message, end the game, or resume.
    fn advance_after_input(&mut self) {
        if self.outcome.is_some() {
            self.mode = if self.pending.is_empty() {
                Mode::GameOver
            } else {
                Mode::Interaction
            };
            return;
        }
        if self.pending.is_empty() {
            self.run_phases();
        } else {
            self.mode = Mode::Interaction;
        }
    }

    fn input_err(&mut self, msg: &str) -> Result<(), String> {
        self.input_error = Some(msg.to_string());
        Err(msg.to_string())
    }

    pub(crate) fn set_outcome(&mut self, end: EndGame) {
        self.outcome = Some(end);
    }
}

/// Interpret a yes/no decision answer; `None` for any other response shape.
fn as_bool(response: Response) -> Option<bool> {
    match response {
        Response::Yes => Some(true),
        Response::No => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
