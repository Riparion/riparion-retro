//! Phase 2 — the Trace. The 450-mile walk home from Natchez to Nashville: a day
//! at a time, choosing pace and company, burning provisions and health, dodging
//! bandits and fording rivers, resting at the stands.

use serde::{Deserialize, Serialize};

use super::interaction::Interaction;
use super::scenario_data::scenario;
use super::state::{GameOverCause, Mode, Pace, Phase, Stand};
use super::{Flow, Game, Resume};

/// Where a Trace day is in its little chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Leg {
    /// Roll the day's trail hazard.
    Hazard,
    /// Settle the day — reach a stand, ford a river.
    Settle,
}

impl Game {
    // ----- The Trace hub -----

    pub fn set_pace(&mut self, pace: Pace) {
        if self.mode == Mode::TraceHub {
            self.state.pace = pace;
        }
    }

    pub fn set_grouped(&mut self, grouped: bool) {
        if self.mode == Mode::TraceHub {
            self.state.grouped = grouped;
        }
    }

    /// Press on for a day's walk.
    pub fn travel_day(&mut self) {
        if self.phase != Phase::Trace || self.mode != Mode::TraceHub {
            return;
        }
        self.add_trace_day();
        if self.outcome.is_some() {
            return;
        }
        self.leg = Some(Leg::Hazard);
        self.resume = Resume::Leg;
        self.run_leg();
    }

    /// Cover a day's ground; eat; wear down. Can end in starvation/disease.
    fn add_trace_day(&mut self) {
        let bonus = self.rng.uniform() * 3.0;
        self.state.miles += self.state.pace.miles_per_day(self.state.has_horse) + bonus;
        self.state.day += 1;

        let cost = self.state.pace.provisions_cost();
        let starving = self.state.provisions < cost;
        if starving {
            self.state.provisions = 0.0;
            self.state.health -= 8.0;
        } else {
            self.state.provisions -= cost;
        }
        self.state.health -= self.state.pace.health_cost();

        if self.state.health <= 0.0 {
            let cause = if starving {
                GameOverCause::Starved
            } else {
                GameOverCause::Disease
            };
            self.die(cause);
        }
    }

    /// Advance the day's chain one stage.
    pub(crate) fn run_leg(&mut self) {
        let flow = match self.leg {
            Some(Leg::Hazard) => {
                self.leg = Some(Leg::Settle);
                self.do_trace_hazard()
            }
            Some(Leg::Settle) => {
                self.leg = None;
                self.resume = Resume::NextDay;
                self.settle_day()
            }
            None => {
                self.resume = Resume::NextDay;
                Flow::Continue
            }
        };
        if let Flow::Continue = flow {
            self.advance();
        }
    }

    /// Deal one trail hazard. Bandits are likelier north of the divide (an arm
    /// `Branch`); traveling grouped thins the ambushes. The arm-to-minigame
    /// binding comes from the scenario.
    fn do_trace_hazard(&mut self) -> Flow {
        let hazards = &scenario().trace.hazards;
        let r1 = self.rng.uniform() * 100.0;
        let mut idx = 0usize;
        for (i, t) in hazards.thresholds.iter().enumerate() {
            if r1 <= *t {
                idx = i + 1;
                break;
            }
        }
        // Company on the road draws off some ambushes.
        if self.state.grouped && hazards.grouped_thins.contains(&idx) && self.rng.one_in(2.0) {
            idx = 0;
        }
        self.run_hazard_arm(&hazards.arms[idx])
    }

    /// Settle the day: reach every stand whose milepost the day's travel passed.
    /// Rest stands pause to their screen (resumed from [`leave_stand`]); the Duck
    /// River crossing runs inline. Looping here means a long day that vaults two
    /// stops can't silently drop the second — in particular the river crossing
    /// can't be skipped by a fast final day on the way into Nashville.
    fn settle_day(&mut self) -> Flow {
        let posts = Stand::posts();
        loop {
            if self.state.stand_idx >= posts.len() {
                return Flow::Continue;
            }
            let (stand, post) = posts[self.state.stand_idx];
            if self.state.miles < post {
                return Flow::Continue;
            }
            self.state.stand_idx += 1;
            match stand {
                Stand::DuckRiver => {
                    if self.state.has_horse {
                        self.message_keyed(
                            scenario().setpieces.duck_horse_msg.clone(),
                            "trace-duck-river",
                        );
                        // Crossing handled; carry on settling any further stand.
                    } else {
                        // A ferryman waits — pay the toll or ford it. The prompt
                        // must resolve before the day can end.
                        self.pending.push_back(Interaction::FerryToll {
                            toll: scenario().setpieces.ferry_toll,
                        });
                        return Flow::Continue;
                    }
                }
                _ => {
                    self.mode = Mode::Stand;
                    return Flow::Pause;
                }
            }
        }
    }

    /// Check arrival/limit at the end of a day, else back to the hub.
    pub(crate) fn next_day(&mut self) {
        if self.state.miles >= scenario().trace.total_miles {
            self.finish_win();
            return;
        }
        if self.state.day >= scenario().trace.max_days {
            self.die(GameOverCause::LostInWoods);
            return;
        }
        self.mode = Mode::TraceHub;
    }

    // ----- Stands -----

    /// The stand most recently reached (for the Stand screen and its cover).
    pub fn current_stand(&self) -> Option<Stand> {
        if self.phase != Phase::Trace {
            return None;
        }
        let i = self.state.stand_idx;
        if i == 0 || i > Stand::ALL.len() {
            return None;
        }
        Some(Stand::ALL[i - 1])
    }

    /// Rest and buy a meal and provisions at the stand.
    pub fn rest_and_resupply(&mut self, cost: f64) {
        if self.mode != Mode::Stand {
            return;
        }
        let sp = &scenario().setpieces;
        self.spend(cost);
        self.state.provisions += sp.rest_provisions;
        self.state.health = (self.state.health + sp.rest_health).min(100.0);
    }

    /// Leave the stand and carry on. First settle any further crossing the day's
    /// travel already passed (e.g. the Duck River lying just beyond a rest stand),
    /// then end the day — so leaving a stand can never vault the river crossing.
    pub fn leave_stand(&mut self) {
        if self.mode != Mode::Stand {
            return;
        }
        match self.settle_day() {
            // Another stop this same day — stay parked on its screen.
            Flow::Pause => {}
            // Nothing (or only an inline crossing) left: run it out to day's end.
            Flow::Continue => self.advance(),
        }
    }
}
