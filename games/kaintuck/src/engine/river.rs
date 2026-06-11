//! Phase 1 — the River. The downstream run from Pittsburgh to Natchez: casting
//! off, the per-leg hazard chain, arriving at each landing, the Falls of the
//! Ohio set-piece, and selling everything at Natchez before the walk home.

use serde::{Deserialize, Serialize};

use super::interaction::Interaction;
use super::prices;
use super::state::{
    Mode, Phase, CINCINNATI, LOUISVILLE, MEMPHIS, NATCHEZ, NUM_GOODS, NUM_RIVER_TOWNS,
    STARTING_PROVISIONS, TOWN_NAMES, TOWN_SLUGS,
};
use super::tasks::{BrigadeTask, QuickTask, SequenceTask, SteadyTask};
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

/// The price the falls pilot charges to take you through.
const PILOT_FEE: f64 = 8.0;

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
    /// first one `r1` falls under selects the hazard (else a clean run).
    fn do_river_hazard(&mut self) -> Flow {
        let r1 = self.rng.uniform() * 100.0;
        const THRESH: [f64; 6] = [10.0, 20.0, 30.0, 42.0, 52.0, 60.0];
        let mut idx = 0usize;
        for (i, t) in THRESH.iter().enumerate() {
            if r1 <= *t {
                idx = i + 1;
                break;
            }
        }
        match idx {
            1 => {
                // Sandbar — hold her off the bar (a deeper draft makes it worse).
                self.begin_steady(SteadyTask::Sandbar);
                Flow::Pause
            }
            2 => {
                // Snag punches at the hull — patch it in the right order.
                self.begin_sequence(SequenceTask::Patch);
                Flow::Pause
            }
            3 => {
                // River pirates close in — draw fast.
                self.message_keyed("River pirates pull alongside!", "pirates");
                self.begin_quick(QuickTask::Pirates);
                Flow::Pause
            }
            4 => {
                // Spoilage — the perishables turn in the heat.
                let rot = (self.rng.ri(15) + 5) as f64 / 100.0;
                let mut lost = false;
                for i in [4usize, 6, 2] {
                    // pork, livestock, flour
                    let n = (self.state.hold[i] as f64 * rot).floor() as i64;
                    if n > 0 {
                        self.state.hold[i] -= n;
                        lost = true;
                    }
                }
                if lost {
                    self.message_keyed(
                        "Heat and damp spoil some of the perishable cargo.",
                        "spoilage",
                    );
                    self.dent_morale(5.0);
                }
                Flow::Continue
            }
            5 => {
                // A hand grumbles and morale dips; sometimes one slips off.
                self.dent_morale(8.0);
                if self.state.crew > 1 && self.rng.one_in(4.0) {
                    self.state.crew -= 1;
                    self.message("A hand slips off the boat at a wooding stop and is gone by morning.");
                } else {
                    self.message("The crew grows surly on the long, slow water.");
                }
                Flow::Continue
            }
            6 => {
                // High water / flood — bail her out.
                self.begin_brigade(BrigadeTask::Bail);
                Flow::Pause
            }
            _ => {
                self.message("The current carries you on. A quiet stretch of river.");
                Flow::Continue
            }
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
            let offered = (self.state.cash * 2.0).max(super::state::STARTING_CREDIT_CAP);
            self.state.credit_cap = self.state.credit_cap.max(offered);
        }
        prices::generate_prices(&mut self.state, &mut self.rng);

        self.message_keyed(format!("You make {}.", TOWN_NAMES[to]), town_cover(to));
        self.river_desertion();

        self.resume = if to == NATCHEZ {
            Resume::Natchez
        } else if to == LOUISVILLE {
            Resume::Falls
        } else {
            Resume::Town
        };
        self.advance();
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

    // ----- The Falls of the Ohio -----

    /// Hire a falls pilot: a sure thing for a fee.
    pub fn falls_pilot(&mut self) {
        if self.mode != Mode::Falls {
            return;
        }
        self.spend(PILOT_FEE);
        self.resume = Resume::Town;
        self.message("A falls pilot takes the steering oar and threads you down the two-mile chute, clean.");
        self.advance();
    }

    /// Run the falls yourself — free, and the steady-hand set-piece.
    pub fn falls_run(&mut self) {
        if self.mode != Mode::Falls {
            return;
        }
        self.resume = Resume::Town;
        self.begin_steady(SteadyTask::FallsRun);
    }

    /// Wait for high water — costs time and frays the crew, but the run is safe.
    pub fn falls_wait(&mut self) {
        if self.mode != Mode::Falls {
            return;
        }
        self.resume = Resume::Town;
        self.dent_morale(12.0);
        if self.state.crew > 1 && self.rng.one_in(3.0) {
            self.state.crew -= 1;
            self.message("You wait days for high water. One hand tires of it and quits — but the swollen river carries you over the falls with ease.");
        } else {
            self.message("You wait for high water. The crew grumbles at the lost days, but the swollen river carries you over the falls with ease.");
        }
        self.advance();
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

    /// Buy a horse for the Trace (also offered at Buzzard Roost).
    pub fn buy_horse(&mut self, price: f64) {
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
        self.state.provisions = STARTING_PROVISIONS;
        self.state.stand_idx = 0;
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
