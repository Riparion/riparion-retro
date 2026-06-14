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

pub mod action;
pub mod gossip;
pub mod interaction;
pub mod ledger;
pub mod market;
pub mod market_link;
pub mod net;
pub mod policy;
pub mod prices;
pub use retro_core::rng;
pub mod scenario_data;
pub mod scoring;
pub mod state;
pub mod tasks;
pub mod world;

// The autonomous economy simulator (Monte-Carlo balance + load harness). Pure,
// but gated off the default build so the wasm UI never compiles it.
#[cfg(feature = "sim")]
pub mod sim;

mod river;
mod trace;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use interaction::{Interaction, Response};
use rng::GameRng;
use scenario_data::scenario;
use state::{Boat, EndGame, GameOverCause, GameState, Mode, Pace, Phase, NUM_GOODS};
use tasks::{MiniTask, SteadyTask};

pub use river::Voyage;
pub use trace::Leg;

use trail_kit::{BanterGate, BanterPhase, EffectTarget, HazardArm};

/// When this many shared-world gossip events are waiting, the crew airs one on a
/// quiet leg even if an authored beat is still eligible — so a busy river drains
/// the feed instead of overflowing it (`gossip::GossipFeed::CAP` is the ceiling).
const GOSSIP_PRESSURE: usize = 8;

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
    GrandTower,
    CaveInRock,
    Natchez,
    TraceHub,
    Leg,
    NextDay,
    /// After the moored hazard's narration drains, launch the self-repair
    /// sequence (then fall back to the town hub).
    SelfRepair,
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
    /// The one minigame currently paused for the player, if any.
    pub pending_task: Option<MiniTask>,
    /// Stake riding on an Under-the-Hill gamble, held while the timing game runs.
    pub pending_stake: f64,
    pub outcome: Option<EndGame>,
    pub rng: GameRng,
    /// Opt-in link to a shared market (the economy sim / multiplayer server).
    /// `None` for the standalone single-player game, in which case pricing and
    /// trading behave exactly as they always have. Never serialized — a save
    /// reloads as an ordinary offline game, and the host re-attaches the link.
    /// See [`market_link`].
    #[serde(skip)]
    pub market_link: Option<market_link::MarketLink>,
    /// Opt-in feed of gossip about *other* traders in the shared world, fed by
    /// the multiplayer client; the crew voices it as banter. `None` offline, so
    /// single-player is unaffected. Never serialized. See [`gossip`].
    #[serde(skip)]
    pub gossip: Option<gossip::GossipFeed>,
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
            pending_task: None,
            pending_stake: 0.0,
            outcome: None,
            rng: GameRng::from_seed(seed),
            market_link: None,
            gossip: None,
        }
    }

    /// Begin at the Pittsburgh boatyard, seeding starting cash and reputation
    /// from a prior run's house ledger (see [`ledger::Carryover`]). A house that
    /// has never run passes [`ledger::Carryover::fresh`] for the clean-slate
    /// scenario stake.
    pub fn begin_with(&mut self, trader: String, carry: ledger::Carryover) {
        let trader = trader.trim().to_string();
        self.state = GameState::new(trader);
        self.state.cash = carry.cash;
        self.state.reputation = carry.reputation;
        self.phase = Phase::River;
        self.pending.clear();
        self.voyage = None;
        self.leg = None;
        self.resume = Resume::Town;
        self.pending_task = None;
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
        let cost = kind.cost() + crew as f64 * scenario().start.crew_wage;
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

    /// The mid quote for `good` at the current town. Normally `state.prices`
    /// (the privately-rolled local mid); when a shared-market link is attached
    /// (sim / multiplayer), the link's authoritative mid shadows it instead. The
    /// spread/reputation math in `buy_price`/`sell_price` is identical either way.
    ///
    /// The link is only trusted when its `town` matches the current `state.town`:
    /// the link's quote is a snapshot for one town, and a client refreshes it
    /// asynchronously, so right after the player moves towns it can briefly lag.
    /// On a mismatch we fall back to the local price rather than quote a stale
    /// town's mid against this town's market (and `record_fill` stamps the trade
    /// with `state.town`, so the host applies it to the correct market).
    fn mid(&self, good: usize) -> f64 {
        match &self.market_link {
            Some(link) if link.town == self.state.town => link.quote.mid[good],
            _ => self.state.prices[good],
        }
    }

    /// The ask (what you pay to buy one unit of `good` here): the mid quote
    /// lifted half a bid/ask spread. The market always costs more to enter than
    /// it returns — see [`Game::sell_price`].
    pub fn buy_price(&self, good: usize) -> f64 {
        self.mid(good) * (1.0 + prices::spread(self.state.town, good) / 2.0)
    }

    /// The bid (what you net selling one unit of `good` here): the mid dropped
    /// half a spread, then nudged ±25% by reputation. The reputation edge applies
    /// to selling only, as before.
    pub fn sell_price(&self, good: usize) -> f64 {
        let bid = self.mid(good) * (1.0 - prices::spread(self.state.town, good) / 2.0);
        (bid * (1.0 + self.state.reputation / 200.0)).max(0.0)
    }

    /// Whether the current town locally produces `good` — drives the "*" flag and
    /// the "locally produced" footnote on the trade screen.
    pub fn produces_locally(&self, good: usize) -> bool {
        prices::is_locally_produced(self.state.town, good)
    }

    /// What the whole hold would actually fetch if sold here right now — every
    /// good at its current bid (spread and reputation folded in). This is the
    /// realizable figure the Natchez cash-out shows, as opposed to the notional
    /// mid-quote [`GameState::cargo_value`](state::GameState::cargo_value).
    pub fn cargo_sale_value(&self) -> f64 {
        (0..state::NUM_GOODS)
            .map(|i| self.state.hold[i] as f64 * self.sell_price(i))
            .sum()
    }

    /// Max units of `good` the player can fund (cash plus boatyard credit) and
    /// fit aboard. Cargo bought past your cash is carried on credit — the whole
    /// trade is to buy cheap on credit upstream and sell high downstream.
    pub fn max_buy(&self, good: usize) -> i64 {
        let price = self.buy_price(good);
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
        let unit_price = self.buy_price(good);
        self.spend(qty as f64 * unit_price);
        self.state.hold[good] += qty;
        self.record_fill(good, market_link::Side::Buy, qty, unit_price);
    }

    /// Sell `qty` of `good` at the current bid (see [`Game::sell_price`]).
    pub fn sell(&mut self, good: usize, qty: i64) {
        let qty = qty.clamp(0, self.state.hold[good]);
        let unit_price = self.sell_price(good);
        self.state.cash += qty as f64 * unit_price;
        self.state.hold[good] -= qty;
        self.record_fill(good, market_link::Side::Sell, qty, unit_price);
    }

    /// Record a trade on the shared-market link, if one is attached. A no-op for
    /// the standalone single-player game and for zero-quantity trades.
    fn record_fill(&mut self, good: usize, side: market_link::Side, qty: i64, unit_price: f64) {
        if qty == 0 {
            return;
        }
        if let Some(link) = &mut self.market_link {
            link.fills.push(market_link::TradeIntent {
                town: self.state.town,
                good,
                side,
                qty,
                unit_price,
            });
        }
    }

    // ----- Moneylender -----

    pub fn max_borrow(&self) -> f64 {
        (self.state.credit_cap - self.state.debt).max(0.0)
    }

    /// The per-leg interest on the boatyard debt, set by the trader's standing:
    /// the scenario's `base_interest` swung ±half its value by reputation in
    /// [-50, 50] (so cheaper when respected, dearer when notorious), clamped to
    /// that band. Shared by the arrival accrual and the moneylender screen so the
    /// quoted rate is the rate actually charged.
    pub fn interest_rate(&self) -> f64 {
        let base = scenario().start.base_interest;
        (base - self.state.reputation * base / 100.0).clamp(base / 2.0, base * 1.5)
    }

    /// The credit cap the Cincinnati/Memphis moneylenders offer on arrival: the
    /// scenario's `credit_multiple` of the trader's cash, scaled ±50% by
    /// reputation in [-50, 50], and never below the flat `credit_cap` floor.
    /// Shared so any "your name gets you up to $X" quote matches what
    /// [`Game::arrive_at_town`] actually grants.
    pub fn credit_offer(&self) -> f64 {
        let start = &scenario().start;
        let rep_factor = 1.0 + self.state.reputation / 100.0; // 0.5 .. 1.5
        (self.state.cash * start.credit_multiple * rep_factor).max(start.credit_cap)
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
                    // Push hard and you reach Colbert's crossing after dark — his
                    // contrary hands won't set you over until daylight, a day lost.
                    // (The penalty amount and copy are scenario data; the "arrived
                    // late" trigger is pace-conditional, so it stays in code.)
                    if self.state.pace == Pace::Hard {
                        let sp = &scenario().setpieces;
                        if sp.ferry_late_days > 0 {
                            self.lose_days(sp.ferry_late_days);
                            self.message(sp.ferry_late_msg.clone());
                        }
                    }
                    self.message(scenario().setpieces.ferry_cross_msg.clone());
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
                        Resume::GrandTower => self.mode = Mode::GrandTower,
                        Resume::CaveInRock => self.mode = Mode::CaveInRock,
                        Resume::Natchez => self.mode = Mode::Natchez,
                        Resume::SelfRepair => {
                            // The moored hazard's lines have shown; now lie up and
                            // mend her. Reset the resume so the sequence's own
                            // resolve lands back at the town hub.
                            self.resume = Resume::Town;
                            self.begin_sequence(tasks::SequenceTask::SelfRepair);
                        }
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

    // ----- Hazard arm dispatch (data-driven) -----

    /// Roll an arm index off a hazard table: a percentile draw selects the first
    /// band it falls under (else arm 0, a clean leg), and when travelling in
    /// company (a convoy on the water, a party on the road) the thinnable
    /// ambushes get a coin-flip chance to be drawn off. Shared by both phases'
    /// hazard rolls so the river and Trace selection logic can't drift apart.
    pub(crate) fn pick_arm_idx(&mut self, hazards: &trail_kit::scenario::HazardTable, in_company: bool) -> usize {
        let r1 = self.rng.uniform() * 100.0;
        let mut idx = 0usize;
        for (i, t) in hazards.thresholds.iter().enumerate() {
            if r1 <= *t {
                idx = i + 1;
                break;
            }
        }
        if in_company && hazards.grouped_thins.contains(&idx) && self.rng.one_in(2.0) {
            idx = 0;
        }
        idx
    }

    /// Carry out a selected hazard arm. Both phases roll an index off their
    /// `HazardTable` and hand the chosen [`HazardArm`] here, so which minigame a
    /// hazard fires (and which clean/special leg it is) lives in the scenario, not
    /// in code.
    pub(crate) fn run_hazard_arm(&mut self, arm: &HazardArm) -> Flow {
        match arm {
            HazardArm::Clean { message, cover } => {
                // A quiet leg is a chance for ambient crew banter about the
                // country you're passing through; only if none is left to hear
                // do we fall back to the flat clean line.
                if !self.queue_banter() {
                    self.narrate_opt(message, cover);
                }
                Flow::Continue
            }
            HazardArm::Minigame {
                outcome,
                message,
                cover,
            } => {
                self.narrate_opt(message, cover);
                self.begin_minigame_for(outcome);
                Flow::Pause
            }
            HazardArm::Special(name) => self.run_special(name),
            HazardArm::Branch {
                past_divide,
                before,
            } => {
                let arm = if self.state.miles >= scenario().trace.divide_at {
                    past_divide
                } else {
                    before
                };
                self.run_hazard_arm(arm)
            }
        }
    }

    /// Narrate an optional arm line, reusing the effect-interpreter's narration
    /// seam so the keyed-vs-plain decision lives in exactly one place.
    fn narrate_opt(&mut self, message: &Option<String>, cover: &Option<String>) {
        if let Some(m) = message {
            EffectTarget::narrate(self, m.clone(), cover.clone());
        }
    }

    /// On a quiet leg, play an ambient crew-banter beat for the current region
    /// if one is left unheard. Selection is deterministic — the first eligible
    /// beat in authored order — and never touches `self.rng`, so it can't shift
    /// the hazard sequence. Returns whether a beat was queued, so the caller can
    /// skip the flat clean line when banter took its place.
    fn queue_banter(&mut self) -> bool {
        // Region is keyed by position: river miles at the leg's origin landing
        // (still `state.town` mid-voyage), or Trace miles walked.
        let (phase, pos) = match self.phase {
            Phase::River => (
                BanterPhase::River,
                scenario().river.towns[self.state.town].milepost,
            ),
            Phase::Trace => (BanterPhase::Trace, self.state.miles),
        };
        // When shared-world chatter is piling up, air some even on a beat-rich
        // leg — so gossip cadence tracks how busy the river is, not the inverse of
        // local authored-beat density (which would let the feed silently overflow
        // exactly where beats are plentiful). Offline this is always false.
        if self.gossip_pending() >= GOSSIP_PRESSURE && self.voice_trader_gossip() {
            return true;
        }
        let beat = scenario().banter.iter().find_map(|pool| {
            if pool.phase != phase || pos < pool.from_mile || pos >= pool.to_mile {
                return None;
            }
            pool.beats.iter().find(|b| {
                !self.state.heard_banter.contains(&b.key) && self.banter_gates_pass(&b.gates)
            })
        });
        let Some(beat) = beat else {
            // No authored beat left for this region — let the crew trade gossip
            // about other traders instead (occasional seasoning: it fills the gaps
            // the hand-authored beats leave).
            return self.voice_trader_gossip();
        };
        for line in &beat.lines {
            self.message(format!("{} {}", line.voice, line.text));
        }
        self.state.heard_banter.insert(beat.key.clone());
        true
    }

    /// Voice one piece of trader gossip from the shared-world feed, if any is
    /// waiting. Composes the line from the scenario's gossip flavor; a no-op
    /// (returns `false`) offline, where `gossip` is `None`. Never touches the RNG.
    ///
    /// Skips (drops) any event whose kind has no phrasing rather than stopping at
    /// it — so a non-composable head-of-line event can't block the rest of the feed.
    fn voice_trader_gossip(&mut self) -> bool {
        while let Some(event) = self.gossip.as_mut().and_then(|f| f.take_oldest()) {
            if let Some((voice, line)) = event.compose() {
                self.message(format!("{voice} {line}"));
                return true;
            }
            // Non-composable (a kind with no authored phrasing): drop and try the
            // next rather than leaving it to stall the feed.
        }
        false
    }

    /// Pending shared-world gossip count (0 offline).
    fn gossip_pending(&self) -> usize {
        self.gossip.as_ref().map_or(0, |f| f.len())
    }

    /// Whether every gate on a banter beat passes against current state.
    fn banter_gates_pass(&self, gates: &[BanterGate]) -> bool {
        gates.iter().all(|g| match g {
            BanterGate::MoraleBelow(t) => self.state.morale < *t,
            BanterGate::MoraleAbove(t) => self.state.morale >= *t,
            BanterGate::Grouped(b) => self.state.grouped == *b,
            // Kaintuck's single "marked" antagonist is Mason (Cave-in-Rock).
            BanterGate::Marked(b) => self.state.crossed_mason == *b,
        })
    }

    /// Dispatch a built-in special arm whose RNG-driven internals stay in Rust.
    fn run_special(&mut self, name: &str) -> Flow {
        match name {
            "spoilage" => {
                self.do_spoilage();
                Flow::Continue
            }
            "crew-grumble" => {
                self.do_crew_grumble();
                Flow::Continue
            }
            "counterfeit" => {
                self.do_counterfeit();
                Flow::Continue
            }
            "moor-mishap" => {
                self.do_moor_mishap();
                Flow::Continue
            }
            other => panic!("unknown hazard special {other}"),
        }
    }

    // ----- Set-piece menu dispatch (data-driven) -----

    /// Carry out a set-piece option by its action tag. The menu *structure* lives
    /// in the scenario; this maps each tag to the existing, tested engine op (so
    /// behaviour is unchanged from when the buttons were hand-wired). Pure UI-nav
    /// actions (`sell-cargo`, `gamble`) are handled by the screen, not here.
    pub fn run_set_piece(&mut self, action: &str, cost: f64) {
        match action {
            "falls-pilot" => self.falls_pilot(cost),
            "falls-run" => self.falls_run(),
            "falls-wait" => self.falls_wait(),
            "gt-treat" => self.grand_tower_treat(cost),
            "gt-duck" => self.grand_tower_duck(),
            "cave-take" => self.cave_take(),
            "cave-hire" => self.cave_hire(cost),
            "cave-run" => self.cave_run(),
            "sell-boat" => self.sell_boat(),
            "buy-horse" => self.buy_horse(cost),
            "set-out" => self.set_out_on_trace(),
            "rest" => self.rest_and_resupply(cost),
            "leave" => self.leave_stand(),
            // `sell-cargo` (→ Trade screen), `gamble` (→ stake entry), and
            // `moneylender` (→ Moneylender screen) are screen navigation,
            // dispatched by the UI — an explicit no-op here.
            "sell-cargo" | "gamble" | "moneylender" => {}
            // Anything else is a scenario/code mismatch; fail loudly like the
            // sibling dispatchers (begin_minigame_for, run_special).
            other => panic!("unknown set-piece action {other}"),
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

    /// Burn `n` days to deliberate delay (a convoy wait, a late ferry). These
    /// count against the speed score the same as any other day.
    pub(crate) fn lose_days(&mut self, n: u32) {
        self.state.extra_days = self.state.extra_days.saturating_add(n);
    }

    pub(crate) fn adjust_reputation(&mut self, d: f64) {
        self.state.reputation = (self.state.reputation + d).clamp(-50.0, 50.0);
    }

    /// Adjust the boat's hull damage by `d`, clamped to 0..100. She wrecks — game
    /// over — on reaching 100. The single chokepoint every damage path funnels
    /// through (the `AdjustBoatDamage` effect, a moored mishap, a botched
    /// self-repair), so the clamp and the wreck threshold live in exactly one
    /// place. A no-op with no boat.
    pub(crate) fn adjust_boat_damage(&mut self, d: f64) {
        let Some(b) = self.state.boat.as_mut() else {
            return;
        };
        b.damage = (b.damage + d).clamp(0.0, 100.0);
        if b.damage >= 100.0 {
            self.die(GameOverCause::BoatWrecked);
        }
    }

    // ----- Endings -----

    fn build_end(&self, cause: GameOverCause, days: i64) -> EndGame {
        let s = &self.state;
        let ending = scenario()
            .ending(cause.key())
            .expect("missing ending for cause");
        let won = ending.won;
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
            cause: ending.message.clone(),
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
        self.state.miles = scenario().trace.total_miles;
        self.outcome = Some(self.build_end(GameOverCause::Victory, days));
        self.mode = Mode::GameOver;
    }

    /// Days elapsed for the reckoning — the river run plus the Trace days.
    fn days_elapsed(&self) -> i64 {
        // ~1 day per 90 river miles reached, plus the counted Trace days, plus
        // any days deliberately lost to convoy waits or a late ferry crossing.
        let river_days = (self.state.river_mile() / 90.0).round() as i64;
        river_days + self.state.day as i64 + self.state.extra_days as i64
    }
}

#[cfg(test)]
mod tests;
