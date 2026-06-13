//! Phase 1 — the River. The downstream run from Pittsburgh to Natchez: casting
//! off, the per-leg hazard chain, arriving at each landing, the Falls of the
//! Ohio set-piece, and selling everything at Natchez before the walk home.

use serde::{Deserialize, Serialize};

use super::interaction::Interaction;
use super::prices;
use super::scenario_data::scenario;
use super::state::{
    Mode, Phase, CAIRO, CINCINNATI, GRAND_TOWER, LOUISVILLE, MEMPHIS, NATCHEZ, NUM_GOODS,
    NUM_RIVER_TOWNS,
    TOWN_SLUGS,
};
use super::tasks::{SteadyTask, TimingTask};
use super::{Flow, Game, Resume};

/// Where a downstream leg is in its little chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiverStage {
    Hazard,
    Arrive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voyage {
    pub to_town: usize,
    pub stage: RiverStage,
}

impl Game {
    // ----- Casting off -----

    /// Cast off downstream toward the next landing. Works from the Pittsburgh
    /// boatyard and from any river-town hub.
    pub fn depart(&mut self) {
        if self.phase != Phase::River {
            return;
        }
        if !matches!(self.mode, Mode::Pittsburgh | Mode::Town) || !self.has_boat() {
            return;
        }
        let to = self.state.town + 1;
        if to >= NUM_RIVER_TOWNS {
            return;
        }
        // Running in company means forming up and keeping the convoy's pace —
        // safer from pirates, but it costs a day on the water.
        if self.state.river_convoy {
            self.lose_days(1);
        }
        self.voyage = Some(Voyage {
            to_town: to,
            stage: RiverStage::Hazard,
        });
        self.continue_voyage();
    }

    /// Advance the downstream chain until a hazard pauses for the player or we
    /// make the next landing.
    pub(crate) fn continue_voyage(&mut self) {
        loop {
            let Some(v) = self.voyage else {
                return;
            };
            match v.stage {
                RiverStage::Hazard => {
                    self.set_river_stage(RiverStage::Arrive);
                    if let Flow::Pause = self.do_river_hazard() {
                        return;
                    }
                }
                RiverStage::Arrive => {
                    let to = v.to_town;
                    self.voyage = None;
                    self.arrive_at_town(to);
                    return;
                }
            }
        }
    }

    fn set_river_stage(&mut self, stage: RiverStage) {
        if let Some(v) = self.voyage.as_mut() {
            v.stage = stage;
        }
    }

    /// Deal one downstream hazard. Thresholds are cumulative percentages; the
    /// first one `r1` falls under selects the arm (else `arms[0]`, a clean run).
    /// The arm itself — which minigame, which special — comes from the scenario.
    fn do_river_hazard(&mut self) -> Flow {
        let river = &scenario().river;
        // A leg's odds come from its destination town's override when present,
        // else the phase-wide table. `to_town` is set while the voyage is in its
        // Hazard stage; fall back to the next landing if a stray call lacks one.
        let to = self.voyage.map_or(self.state.town, |v| v.to_town);
        let hazards = river.towns[to].hazards.as_ref().unwrap_or(&river.hazards);
        // Sailing in company draws off some pirates (the river side of the
        // grouped thinning; only costs an RNG draw when convoyed).
        let idx = self.pick_arm_idx(hazards, self.state.river_convoy);
        self.run_hazard_arm(&hazards.arms[idx])
    }

    /// Set whether the next leg is run in company (a convoy). Settable only at a
    /// river landing, before casting off — the river-side mirror of `set_grouped`.
    pub fn set_river_convoy(&mut self, convoy: bool) {
        if self.mode == Mode::Town || self.mode == Mode::Pittsburgh {
            self.state.river_convoy = convoy;
        }
    }

    /// Special arm: the perishables turn in the heat (pork, livestock, flour).
    pub(crate) fn do_spoilage(&mut self) {
        let rot = (self.rng.ri(15) + 5) as f64 / 100.0;
        let mut lost = false;
        for i in [4usize, 6, 2] {
            let n = (self.state.hold[i] as f64 * rot).floor() as i64;
            if n > 0 {
                self.state.hold[i] -= n;
                lost = true;
            }
        }
        if lost {
            self.message_keyed("Heat and damp spoil some of the perishable cargo.", "spoilage");
            self.dent_morale(5.0);
        }
    }

    /// Special arm: a river sharper passes counterfeit coin (the trade of the
    /// Stack Island and Arkansas gangs). A slice of the purse turns out bad.
    pub(crate) fn do_counterfeit(&mut self) {
        // 8–15% of the purse, capped so it stings without gutting a fat run.
        let frac = (self.rng.ri(8) + 8) as f64 / 100.0;
        let loss = (self.state.cash * frac).min(40.0);
        if loss >= 1.0 {
            self.state.cash -= loss;
            self.adjust_reputation(-4.0);
            self.dent_morale(5.0);
            self.message_keyed(
                "A sharp-dressed trader buys a parcel at the landing and pays in good silver — or so it looks. Days on, the coin rings false on the scales: you took counterfeit, and you're out the money.",
                "counterfeit",
            );
        } else {
            self.message_keyed(
                "A sharp-dressed trader tries to pass you counterfeit coin at the landing. With little in your purse to tempt him, you wave him off.",
                "counterfeit",
            );
        }
    }

    /// Special arm: a hand grumbles and morale dips; sometimes one slips off.
    pub(crate) fn do_crew_grumble(&mut self) {
        self.dent_morale(8.0);
        if self.state.crew > 1 && self.rng.one_in(4.0) {
            self.state.crew -= 1;
            self.message("A hand slips off the boat at a wooding stop and is gone by morning.");
        } else {
            self.message("The crew grows surly on the long, slow water.");
        }
    }

    /// Make a landing: tally interest, regenerate the market, roll for desertion,
    /// then surface the right hub (Falls at Louisville, Natchez at the end).
    fn arrive_at_town(&mut self, to: usize) {
        self.state.town = to;
        self.state.morale = (self.state.morale + 5.0).min(100.0);
        // Interest accrues on the boatyard credit each leg (no flooring, so small
        // debts still grow rather than sitting interest-free under $20).
        self.state.debt += self.state.debt * 0.05;
        // Cincinnati and Memphis moneylenders extend more credit — only ever
        // raising the cap, never snapping it back below an outstanding debt.
        if to == CINCINNATI || to == MEMPHIS {
            let offered = (self.state.cash * 2.0).max(scenario().start.credit_cap);
            self.state.credit_cap = self.state.credit_cap.max(offered);
        }
        prices::generate_prices(&mut self.state, &mut self.rng);

        self.message_keyed(format!("You make {}.", self.state.town_name()), town_cover(to));
        self.queue_market_reality(to);
        self.river_desertion();

        self.resume = if to == NATCHEZ {
            Resume::Natchez
        } else if to == LOUISVILLE {
            Resume::Falls
        } else if to == GRAND_TOWER {
            Resume::GrandTower
        } else if to == CAIRO {
            // Cave-in-Rock sits just above Cairo — the relay-pilot con greets you
            // on the approach, then the normal Cairo landing follows.
            Resume::CaveInRock
        } else {
            Resume::Town
        };
        self.advance();
    }

    /// One-time *reality* line at a landing: the dock's actually-cheap and
    /// actually-dear goods, composed straight from the town's market bias so the
    /// spoken truth can never drift from the prices the player is about to see.
    /// The *rumor* heard on the leg in may have exaggerated or missed it; the
    /// wharf factor sets it straight. Plays at most once per town (recorded in the
    /// same `heard_banter` set as ambient beats), and never consumes RNG.
    fn queue_market_reality(&mut self, town: usize) {
        let key = format!("market-reality:{}", scenario().river.towns[town].slug);
        if self.state.heard_banter.contains(&key) {
            return;
        }
        if let Some(line) = market_reality_line(town) {
            self.message(line);
            self.state.heard_banter.insert(key);
        }
    }

    /// At the dock, a low-morale crew may lose a hand who wants paying off.
    fn river_desertion(&mut self) {
        if self.state.crew > 0 && self.state.morale < 40.0 {
            let n = (self.state.morale / 10.0).max(2.0);
            if self.rng.one_in(n) {
                let pay = self.state.crew as f64 * 2.0 + 4.0;
                self.pending.push_back(Interaction::CrewLeaves { pay });
            }
        }
    }

    /// Shared preamble for a river set-piece option: bail unless we're actually
    /// on that set-piece's screen, and queue the return to the town hub once the
    /// option resolves. Returns whether the caller should proceed. Every set-piece
    /// option (Falls, Grand Tower, Cave-in-Rock) opens with this.
    fn enter_setpiece(&mut self, mode: Mode) -> bool {
        if self.mode != mode {
            return false;
        }
        self.resume = Resume::Town;
        true
    }

    // ----- The Falls of the Ohio -----

    /// Hire a falls pilot: a sure thing for a fee.
    pub fn falls_pilot(&mut self, fee: f64) {
        if !self.enter_setpiece(Mode::Falls) {
            return;
        }
        self.spend(fee);
        self.message(scenario().setpieces.falls_pilot_msg.clone());
        self.advance();
    }

    /// Run the falls yourself — free, and the steady-hand set-piece.
    pub fn falls_run(&mut self) {
        if !self.enter_setpiece(Mode::Falls) {
            return;
        }
        self.begin_steady(SteadyTask::FallsRun);
    }

    /// Wait for high water — costs time and frays the crew, but the run is safe.
    pub fn falls_wait(&mut self) {
        if !self.enter_setpiece(Mode::Falls) {
            return;
        }
        self.dent_morale(12.0);
        if self.state.crew > 1 && self.rng.one_in(3.0) {
            self.state.crew -= 1;
            self.message("You wait days for high water. One hand tires of it and quits — but the swollen river carries you over the falls with ease.");
        } else {
            self.message("You wait for high water. The crew grumbles at the lost days, but the swollen river carries you over the falls with ease.");
        }
        self.advance();
    }

    // ----- Grand Tower: the rivermen's initiation -----

    /// Stand a treat: buy the round and pass with your dignity. Costs a little
    /// cash, but the crew warms to a captain who knows the custom.
    pub fn grand_tower_treat(&mut self, fee: f64) {
        if !self.enter_setpiece(Mode::GrandTower) {
            return;
        }
        self.spend(fee);
        self.state.morale = (self.state.morale + 6.0).min(100.0);
        self.message("You stand the round at the Grand Tower. The rivermen drink your health and wave you past — and your own crew warms to a captain who knows the custom.");
        self.advance();
    }

    /// Refuse the treat and take the ducking: free, and a wry rite of passage —
    /// you come up a true riverman, if a soggy and sore one.
    pub fn grand_tower_duck(&mut self) {
        if !self.enter_setpiece(Mode::GrandTower) {
            return;
        }
        self.dent_morale(8.0);
        self.adjust_reputation(2.0);
        self.message("You won't pay, so they haul you over the side and duck you in the Mississippi to a chorus of laughter. You come up sputtering — a true riverman now, and known for it.");
        self.advance();
    }

    // ----- Cave-in-Rock: the relay-pilot con -----

    /// Take the stranger's free pilot offer — but read him first. He's the
    /// pirates' man; the relay-pilot con (RESEARCH_PIRATES §4) turns on whether you
    /// catch his tell in time. A clean read waves him off safely; a misread with a
    /// hold worth taking drops you onto the bar and into the boarding. The
    /// cargo-value gate (a fat hold is worth grounding for, a poor one isn't) lives
    /// in `resolve_timing`'s CaveTell arm.
    pub fn cave_take(&mut self) {
        if !self.enter_setpiece(Mode::CaveInRock) {
            return;
        }
        self.begin_timing(TimingTask::CaveTell);
    }

    /// Hire your own pilot — the honest hand the *Navigator* names — for a fee.
    pub fn cave_hire(&mut self, fee: f64) {
        if !self.enter_setpiece(Mode::CaveInRock) {
            return;
        }
        self.spend(fee);
        self.message("You wave off the stranger and hire the pilot the Navigator names. He threads the serpentine channel below the cave and sets you clear of the shoals.");
        self.advance();
    }

    /// Run the shoals below the cave yourself — free, and a steady-hand chute.
    pub fn cave_run(&mut self) {
        if !self.enter_setpiece(Mode::CaveInRock) {
            return;
        }
        self.begin_steady(SteadyTask::CaveRun);
    }

    // ----- Natchez: sell everything, then the Trace -----

    /// Break the flatboat up for lumber — it can't go back upstream.
    pub fn sell_boat(&mut self) {
        if self.mode != Mode::Natchez {
            return;
        }
        if let Some(boat) = self.state.boat.take() {
            self.state.cash += boat.lumber_value();
        }
    }

    /// Stake `amount` on a night Under-the-Hill — the timing-bar gamble.
    pub fn gamble(&mut self, amount: f64) {
        if self.mode != Mode::Natchez {
            return;
        }
        let stake = amount.clamp(0.0, self.state.cash);
        if stake <= 0.0 {
            return;
        }
        // Escrow the stake now so the bet is genuinely at risk the moment it is
        // laid — abandoning the night (e.g. a refresh mid-game) forfeits it
        // rather than handing back a free retry from a full purse.
        self.state.cash -= stake;
        self.pending_stake = stake;
        self.begin_timing(super::tasks::TimingTask::Gamble);
    }

    /// Buy a horse for the Trace — at Natchez or traded at a stand (Buzzard
    /// Roost). Guarded to those two screens so a stray dispatch can't conjure one
    /// elsewhere (the old `stand_buy_horse` carried the stand half of this guard).
    pub fn buy_horse(&mut self, price: f64) {
        if !matches!(self.mode, Mode::Natchez | Mode::Stand) {
            return;
        }
        if self.state.has_horse {
            return;
        }
        if price <= self.state.cash {
            self.state.cash -= price;
            self.state.has_horse = true;
        }
    }

    /// Set out up the Natchez Trace. Liquidates anything still aboard at the
    /// current Natchez prices, breaks up the boat, and switches to the Trace.
    pub fn set_out_on_trace(&mut self) {
        if self.mode != Mode::Natchez {
            return;
        }
        for i in 0..NUM_GOODS {
            if self.state.hold[i] > 0 {
                self.sell(i, self.state.hold[i]);
            }
        }
        if let Some(boat) = self.state.boat.take() {
            self.state.cash += boat.lumber_value();
        }
        self.enter_trace();
    }

    /// Flip to the Trace phase and reset for the walk.
    pub(crate) fn enter_trace(&mut self) {
        self.state.crew_at_natchez = self.state.crew;
        self.phase = Phase::Trace;
        self.state.miles = 0.0;
        self.state.day = 0;
        self.state.health = 100.0;
        self.state.provisions = scenario().start.provisions;
        self.state.stand_idx = 0;
        // The river convoy is a water-only concept; clear it so it can't leak
        // into the Trace's `grouped()` and silently halve bandit losses on foot.
        self.state.river_convoy = false;
        self.voyage = None;
        self.leg = None;
        self.resume = Resume::TraceHub;
        self.mode = Mode::TraceHub;
    }
}

// Cover-art slug for a landing, off the shared TOWN_SLUGS table.
fn town_cover(to: usize) -> String {
    format!("town-{}", TOWN_SLUGS[to.min(NUM_RIVER_TOWNS - 1)])
}

/// Compose the dock's *reality* line for `town` from its market bias — the
/// strongest-discounted good (cheap) and the strongest-premium good (dear), in
/// the wharf factor's voice. `None` when the town carries no bias (e.g. Natchez,
/// whose premium lives in the distance gradient, not a local craving). Pure and
/// RNG-free, so it can be asserted directly against the schema in tests.
pub(crate) fn market_reality_line(town: usize) -> Option<String> {
    let market = &scenario().river.towns[town].market;
    let cheapest = market
        .iter()
        .filter(|b| b.supply > 0.0)
        .max_by(|a, b| a.supply.total_cmp(&b.supply));
    let dearest = market
        .iter()
        .filter(|b| b.demand > 0.0)
        .max_by(|a, b| a.demand.total_cmp(&b.demand));
    let mut parts: Vec<String> = Vec::new();
    if let Some(b) = cheapest {
        let good = b.good.to_lowercase();
        parts.push(match &b.note {
            Some(note) => format!("Cheap {good} here — {note}."),
            None => format!("Cheap {good} here; they make it by the boatload."),
        });
    }
    if let Some(b) = dearest {
        let good = b.good.to_lowercase();
        parts.push(match &b.note {
            Some(note) => format!("Scarce {good} — {note}; they'll pay dear for it."),
            None => format!("Scarce {good} here; they'll pay dear for it."),
        });
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("A wharf factor talks shop: {}", parts.join(" ")))
}
