//! Pure game engine — no UI or platform dependencies. The UI owns a `Game` in a
//! signal and calls these methods; everything here is host-testable.
//!
//! ## Two phases, one struct
//!
//! [`Phase::River`] is a downstream cargo-trading run (Pittsburgh → Natchez): you
//! build a flatboat, load cargo, and ride the current down through a chain of
//! river hazards, selling along the way. At Natchez you sell everything, break
//! the boat up for lumber, and step onto the [`Phase::Trace`] — a distance walk
//! home to Nashville with cash in your pockets and bandits on every side.
//!
//! River legs run a [`Voyage`] hazard chain (taipan-style); Trace days run a
//! [`Leg`] chain (oregon-style). Both narrate through the [`Interaction`] queue
//! and can interrupt for a minigame; [`Resume`] records where to pick back up
//! when the queue drains, so a refresh resumes exactly in place.

pub mod interaction;
pub mod prices;
pub use retro_kit::rng;
pub mod scoring;
pub mod state;
pub mod tasks;

mod river;
mod trace;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use interaction::{Interaction, Response};
use rng::GameRng;
use state::{Boat, EndGame, GameOverCause, GameState, Mode, Phase, NUM_GOODS};
use tasks::{BrigadeTask, CrowdTask, QuickTask, SequenceTask, SteadyTask, TimingTask};

pub use river::Voyage;
pub use trace::Leg;

/// Whether a hazard handler paused for a minigame/decision or ran straight through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    Pause,
}

/// Where to land once the message queue drains and no leg/voyage is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resume {
    Town,
    Falls,
    Natchez,
    TraceHub,
    Leg,
    NextDay,
}

/// The complete game: world state, phase, UI mode, pending prompts, the active
/// leg chains, and the RNG. Fully serializable so a refresh resumes in place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub state: GameState,
    pub phase: Phase,
    pub mode: Mode,
    pub pending: VecDeque<Interaction>,
    /// River downstream-leg chain, while afloat.
    pub voyage: Option<Voyage>,
    /// Trace day chain, while walking.
    pub leg: Option<Leg>,
    pub resume: Resume,
    pub pending_steady: Option<SteadyTask>,
    pub pending_quick: Option<QuickTask>,
    pub pending_crowd: Option<CrowdTask>,
    pub pending_timing: Option<TimingTask>,
    pub pending_sequence: Option<SequenceTask>,
    pub pending_brigade: Option<BrigadeTask>,
    /// Stake riding on an Under-the-Hill gamble, held while the timing game runs.
    pub pending_stake: f64,
    pub outcome: Option<EndGame>,
    pub rng: GameRng,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            state: GameState::new(String::new()),
            phase: Phase::River,
            mode: Mode::Splash,
            pending: VecDeque::new(),
            voyage: None,
            leg: None,
            resume: Resume::Town,
            pending_steady: None,
            pending_quick: None,
            pending_crowd: None,
            pending_timing: None,
            pending_sequence: None,
            pending_brigade: None,
            pending_stake: 0.0,
            outcome: None,
            rng: GameRng::from_seed(seed),
        }
    }

    /// Begin a fresh game at the Pittsburgh boatyard.
    pub fn begin(&mut self, trader: String) {
        let trader = trader.trim().to_string();
        self.state = GameState::new(trader);
        self.phase = Phase::River;
        self.pending.clear();
        self.voyage = None;
        self.leg = None;
        self.resume = Resume::Town;
        self.pending_steady = None;
        self.pending_quick = None;
        self.pending_crowd = None;
        self.pending_timing = None;
        self.pending_sequence = None;
        self.pending_brigade = None;
        self.pending_stake = 0.0;
        self.outcome = None;
        // Pittsburgh prices for the initial cargo buy.
        prices::generate_prices(&mut self.state, &mut self.rng);
        self.mode = Mode::Pittsburgh;
    }

    // ----- Pittsburgh: build & outfit -----

    /// Commission a boat and hire a crew, paying with cash (and boatyard credit
    /// up to the cap). Returns an error the UI can show if you can't cover it.
    pub fn build(&mut self, kind: state::BoatKind, crew: i64) -> Result<(), String> {
        if self.mode != Mode::Pittsburgh {
            return Ok(());
        }
        let crew = crew.clamp(1, 5);
        let cost = kind.cost() + crew as f64 * state::CREW_WAGE;
        let available = self.state.cash + (self.state.credit_cap - self.state.debt).max(0.0);
        if cost > available {
            return Err(format!(
                "That's ${} and you can raise only ${}. Pick a smaller boat or less crew.",
                cost as i64, available as i64
            ));
        }
        // Spend cash first, then draw the rest on credit.
        if cost <= self.state.cash {
            self.state.cash -= cost;
        } else {
            let on_credit = cost - self.state.cash;
            self.state.cash = 0.0;
            self.state.debt += on_credit;
        }
        self.state.boat = Some(Boat::new(kind));
        self.state.crew = crew;
        self.state.morale = 100.0;
        Ok(())
    }

    /// Whether the player has built a boat (gates the cargo-buy and cast-off).
    pub fn has_boat(&self) -> bool {
        self.state.boat.is_some()
    }

    // ----- Trading (engine re-clamps; UI input is advisory) -----

    /// Max units of `good` the player can fund (cash plus boatyard credit) and
    /// fit aboard. Cargo bought past your cash is carried on credit — the whole
    /// trade is to buy cheap on credit upstream and sell high downstream.
    pub fn max_buy(&self, good: usize) -> i64 {
        let price = self.state.prices[good];
        if price <= 0.0 {
            return 0;
        }
        let funds = self.state.cash + (self.state.credit_cap - self.state.debt).max(0.0);
        let by_funds = (funds / price).floor() as i64;
        let by_hold = self.state.free_hold().max(0) / state::GOOD_UNITS[good];
        by_funds.min(by_hold).max(0)
    }

    pub fn buy(&mut self, good: usize, qty: i64) {
        let qty = qty.clamp(0, self.max_buy(good));
        self.spend(qty as f64 * self.state.prices[good]);
        self.state.hold[good] += qty;
    }

    /// Sell `qty` of `good`. Reputation nudges the realized price ±25%.
    pub fn sell(&mut self, good: usize, qty: i64) {
        let qty = qty.clamp(0, self.state.hold[good]);
        let price = self.state.prices[good] * (1.0 + self.state.reputation / 200.0);
        self.state.cash += qty as f64 * price.max(0.0);
        self.state.hold[good] -= qty;
    }

    // ----- Moneylender -----

    pub fn max_borrow(&self) -> f64 {
        (self.state.credit_cap - self.state.debt).max(0.0)
    }

    pub fn borrow(&mut self, amount: f64) {
        let amount = amount.clamp(0.0, self.max_borrow());
        self.state.cash += amount;
        self.state.debt += amount;
    }

    pub fn repay(&mut self, amount: f64) {
        let amount = amount.clamp(0.0, self.state.cash.min(self.state.debt));
        self.state.cash -= amount;
        self.state.debt -= amount;
    }

    // ----- Interactions -----

    /// Apply the player's answer to the queue head, then move along.
    pub fn resolve(&mut self, response: Response) {
        if self.mode != Mode::Interaction {
            return;
        }
        let Some(it) = self.pending.pop_front() else {
            self.after_queue();
            return;
        };
        match it {
            Interaction::Message { .. } => {}
            Interaction::CrewLeaves { pay } => {
                if response == Response::Yes {
                    let pay = pay.min(self.state.cash);
                    self.state.cash -= pay;
                    self.state.morale = (self.state.morale + 8.0).min(100.0);
                } else if self.state.crew > 0 {
                    self.state.crew -= 1;
                    self.state.morale = (self.state.morale - 15.0).max(0.0);
                    self.message("The hand walks off with his pay grumbling — the rest take note.");
                }
            }
            Interaction::FerryToll { toll } => {
                if response == Response::Yes {
                    self.spend(toll.min(self.state.cash));
                    self.message("The ferryman poles you across the Duck River, dry and whole.");
                    // crossing done; fall through to advance
                } else {
                    // Ford it yourself — the steady-hand crossing.
                    self.begin_steady(SteadyTask::DuckFord);
                    return;
                }
            }
        }
        if self.pending.is_empty() {
            self.after_queue();
        } else {
            self.mode = Mode::Interaction;
        }
    }

    pub(crate) fn message(&mut self, text: impl Into<String>) {
        self.pending.push_back(Interaction::Message {
            text: text.into(),
            cover: None,
        });
    }

    pub(crate) fn message_keyed(&mut self, text: impl Into<String>, cover: impl Into<String>) {
        self.pending.push_back(Interaction::Message {
            text: text.into(),
            cover: Some(cover.into()),
        });
    }

    /// Show queued messages, or — if none — resume immediately.
    pub(crate) fn advance(&mut self) {
        if self.pending.is_empty() {
            self.after_queue();
        } else {
            self.mode = Mode::Interaction;
        }
    }

    /// The queue drained: resume whatever was interrupted, per phase.
    pub(crate) fn after_queue(&mut self) {
        if self.outcome.is_some() {
            self.mode = Mode::GameOver;
            return;
        }
        match self.phase {
            Phase::River => {
                if self.voyage.is_some() {
                    self.continue_voyage();
                } else {
                    match self.resume {
                        Resume::Falls => self.mode = Mode::Falls,
                        Resume::Natchez => self.mode = Mode::Natchez,
                        _ => self.mode = Mode::Town,
                    }
                }
            }
            Phase::Trace => {
                if self.leg.is_some() {
                    self.run_leg();
                } else {
                    match self.resume {
                        Resume::NextDay => self.next_day(),
                        Resume::Leg => self.run_leg(),
                        _ => self.mode = Mode::TraceHub,
                    }
                }
            }
        }
    }

    // ----- Shared helpers -----

    /// A per-encounter PRNG seed derived from current progress — varies between
    /// encounters yet stays deterministic for a save, and never perturbs the
    /// game's own RNG stream. `salt` keeps co-located minigames distinct.
    pub fn encounter_seed(&self, salt: u64) -> u64 {
        let progress = match self.phase {
            Phase::River => self.state.town as u64,
            Phase::Trace => self.state.day as u64,
        };
        progress
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ self.state.miles.to_bits()
            ^ self.state.cash.to_bits()
            ^ salt
    }

    /// Pay `amount` out of cash, the overflow onto debt (a forced expense).
    pub(crate) fn spend(&mut self, amount: f64) {
        let amount = amount.max(0.0);
        if amount <= self.state.cash {
            self.state.cash -= amount;
        } else {
            self.state.debt += amount - self.state.cash;
            self.state.cash = 0.0;
        }
    }

    /// Lose `frac` (0..1) of the cargo aboard, spread across the goods, and
    /// return the rough market value of what was lost (for narration).
    pub(crate) fn lose_cargo_fraction(&mut self, frac: f64) -> f64 {
        let frac = frac.clamp(0.0, 1.0);
        let mut lost_value = 0.0;
        for i in 0..NUM_GOODS {
            let lost = (self.state.hold[i] as f64 * frac).floor() as i64;
            lost_value += lost as f64 * self.state.prices[i];
            self.state.hold[i] -= lost;
        }
        lost_value
    }

    /// Drop crew morale by `d`, clamped to 0.
    pub(crate) fn dent_morale(&mut self, d: f64) {
        self.state.morale = (self.state.morale - d).max(0.0);
    }

    pub(crate) fn adjust_reputation(&mut self, d: f64) {
        self.state.reputation = (self.state.reputation + d).clamp(-50.0, 50.0);
    }

    // ----- Endings -----

    fn build_end(&self, cause: GameOverCause, days: i64) -> EndGame {
        let s = &self.state;
        let won = cause.won();
        let leftover = s.cash - s.debt;
        let miles_total = s.total_miles(self.phase);
        let crew = s.crew_at_natchez.max(s.crew);
        let score = scoring::score(
            won,
            miles_total,
            days,
            leftover,
            crew,
            s.reputation,
            s.robbed,
        );
        EndGame {
            won,
            cause: cause.message().to_string(),
            cause_kind: cause,
            score,
            rank: scoring::rank(score).to_string(),
            cash: s.cash.max(0.0) as i64,
            debt: s.debt.max(0.0) as i64,
            crew_survived: crew,
            reputation: s.reputation as i64,
            robbed: s.robbed,
            miles: miles_total as i64,
            days,
            recorded: false,
        }
    }

    /// End the journey in failure with `cause`.
    pub(crate) fn die(&mut self, cause: GameOverCause) {
        let days = self.days_elapsed();
        self.outcome = Some(self.build_end(cause, days));
        self.mode = Mode::GameOver;
    }

    /// You walked into Nashville.
    pub(crate) fn finish_win(&mut self) {
        let days = self.days_elapsed();
        self.state.miles = state::TRACE_MILES;
        self.outcome = Some(self.build_end(GameOverCause::Victory, days));
        self.mode = Mode::GameOver;
    }

    /// Days elapsed for the reckoning — the river run plus the Trace days.
    fn days_elapsed(&self) -> i64 {
        // ~1 day per 90 river miles reached, plus the counted Trace days.
        let river_days = (self.state.river_mile() / 90.0).round() as i64;
        river_days + self.state.day as i64
    }
}

#[cfg(test)]
mod tests;
