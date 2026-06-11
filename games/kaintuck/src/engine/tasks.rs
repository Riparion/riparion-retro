//! The minigame layer: which catastrophe put up each minigame (held while it
//! runs so the resolve applies the right effect), the `begin_*` launchers, and
//! the `resolve_*` methods the thin UI wrappers route their results into. Every
//! resolve ends by calling [`Game::advance`], which resumes the right leg/voyage
//! chain for the current phase.

use serde::{Deserialize, Serialize};

use super::state::{fmt_money, GameOverCause, Mode};
use super::Game;

/// Which strain put up the steady-hand trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SteadyTask {
    /// Holding the boat off a sandbar (River).
    Sandbar,
    /// Running the Falls of the Ohio (River).
    FallsRun,
    /// A swamp crossing on the Trace.
    Swamp,
    /// Fording the Duck River on foot (Trace).
    DuckFord,
}

/// Which ambush put up the quick-draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuickTask {
    /// River pirates.
    Pirates,
    /// Sam Mason's gang on the Trace.
    Mason,
    /// The Harpe brothers, north of the divide.
    Harpe,
}

/// Which strain put up the crowd-threading route game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrowdTask {
    /// Keeping to the Trace among the tangle of side trails.
    SideTrail,
}

/// Which strain put up the timing-bar game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimingTask {
    /// A night Under-the-Hill at Natchez.
    Gamble,
    /// Measuring out a dose of medicine against swamp fever.
    Dose,
}

/// Which strain put up the sequence (order-memory) game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SequenceTask {
    /// Patching a snagged hull (River).
    Patch,
}

/// Which strain put up the bucket-brigade game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrigadeTask {
    /// Bailing a flooded boat (River).
    Bail,
}

impl Game {
    // ----- launchers -----

    pub(crate) fn begin_steady(&mut self, task: SteadyTask) {
        self.pending_steady = Some(task);
        self.mode = Mode::Steady;
    }
    pub(crate) fn begin_quick(&mut self, task: QuickTask) {
        self.pending_quick = Some(task);
        self.mode = Mode::Quick;
    }
    pub(crate) fn begin_crowd(&mut self, task: CrowdTask) {
        self.pending_crowd = Some(task);
        self.mode = Mode::Crowd;
    }
    pub(crate) fn begin_timing(&mut self, task: TimingTask) {
        self.pending_timing = Some(task);
        self.mode = Mode::Timing;
    }
    pub(crate) fn begin_sequence(&mut self, task: SequenceTask) {
        self.pending_sequence = Some(task);
        self.mode = Mode::Sequence;
    }
    pub(crate) fn begin_brigade(&mut self, task: BrigadeTask) {
        self.pending_brigade = Some(task);
        self.mode = Mode::Brigade;
    }

    // ----- resolves -----

    /// Steady-hand trace. `steady` = held on target past the threshold;
    /// `accuracy` (0..1) is the fraction of the run on target.
    pub fn resolve_steady(&mut self, steady: bool, accuracy: f64) {
        if self.mode != Mode::Steady {
            return;
        }
        let Some(task) = self.pending_steady.take() else {
            return;
        };
        let drift = (1.0 - accuracy).clamp(0.0, 1.0);
        match task {
            SteadyTask::Sandbar => {
                if steady {
                    self.message("You feel her touch the bar and ease her off without losing a thing.");
                } else {
                    let lost = self.lose_cargo_fraction(0.05 + 0.08 * drift);
                    self.dent_morale(8.0);
                    self.message(format!(
                        "She grinds onto a sandbar — you work her free, but {} of cargo is lost overboard.",
                        fmt_money(lost)
                    ));
                }
            }
            SteadyTask::FallsRun => {
                if accuracy < 0.15 {
                    self.die(GameOverCause::BoatWrecked);
                } else if steady {
                    self.adjust_reputation(5.0);
                    self.message("You run the two-mile chute clean as a whistle. Word of it travels downriver ahead of you.");
                } else {
                    let lost = self.lose_cargo_fraction(0.10 + 0.15 * drift);
                    self.dent_morale(15.0);
                    self.message(format!(
                        "You scrape the rocks coming through the falls — {} of cargo goes into the river.",
                        fmt_money(lost)
                    ));
                }
            }
            SteadyTask::Swamp => {
                if steady {
                    self.message("You pick a firm line through the swamp and come out the far side dry.");
                } else {
                    self.state.health -= 10.0;
                    self.state.provisions = (self.state.provisions - 5.0).max(0.0);
                    self.state.miles -= 5.0;
                    self.message("The swamp nearly swallows you — you lose ground, provisions, and strength.");
                    if self.state.health <= 0.0 {
                        self.die(GameOverCause::Disease);
                    }
                }
            }
            SteadyTask::DuckFord => {
                if !steady && accuracy < 0.2 {
                    self.die(GameOverCause::Drowned);
                } else if steady {
                    self.message("You wade the Duck River, holding your footing against the current — wet, but whole.");
                } else {
                    self.state.health -= 10.0;
                    self.state.provisions = (self.state.provisions - 5.0).max(0.0);
                    self.message("The current nearly takes you crossing the Duck River. You crawl out the far bank half-drowned.");
                    if self.state.health <= 0.0 {
                        self.die(GameOverCause::Disease);
                    }
                }
            }
        }
        self.advance();
    }

    /// Quick-draw. `reaction_secs` is how long the draw took; `hit` whether the
    /// right target was struck.
    pub fn resolve_quick(&mut self, reaction_secs: f64, hit: bool) {
        if self.mode != Mode::Quick {
            return;
        }
        let Some(task) = self.pending_quick.take() else {
            return;
        };
        let good = hit && reaction_secs <= 1.0;
        let slow = hit && reaction_secs > 1.0;
        match task {
            QuickTask::Pirates => {
                if good {
                    self.adjust_reputation(3.0);
                    self.message("Quick on the trigger — the pirates think better of it and sheer off.");
                } else {
                    let mut frac = if slow { 0.15 } else { 0.30 };
                    if self.state.grouped {
                        frac /= 2.0;
                    }
                    let lost = self.lose_cargo_fraction(frac);
                    self.adjust_reputation(-10.0);
                    self.dent_morale(10.0);
                    self.message(format!(
                        "The pirates board you and make off with {} of cargo before you beat them back.",
                        fmt_money(lost)
                    ));
                }
            }
            QuickTask::Mason => {
                if good {
                    self.adjust_reputation(5.0);
                    self.message("You outdraw Mason's man and they melt back into the brush. Few can say as much.");
                } else {
                    self.state.robbed = true;
                    let frac = if self.state.grouped { 0.2 } else { 0.4 };
                    let loss = (self.state.cash * frac).floor();
                    self.state.cash -= loss;
                    self.state.health -= if slow { 10.0 } else { 15.0 };
                    if self.state.health <= 0.0 {
                        self.die(GameOverCause::BanditMurder);
                    } else {
                        self.message(format!(
                            "Sam Mason's gang robs you of {} and leaves you bleeding on the Trace.",
                            fmt_money(loss)
                        ));
                    }
                }
            }
            QuickTask::Harpe => {
                if good {
                    self.adjust_reputation(6.0);
                    self.message("You break away from the Harpes — a thing most men don't live to tell.");
                } else {
                    self.state.robbed = true;
                    let frac = if self.state.grouped { 0.3 } else { 0.6 };
                    let loss = (self.state.cash * frac).floor();
                    self.state.cash -= loss;
                    self.state.health -= if slow { 15.0 } else { 20.0 };
                    if self.state.health <= 0.0 {
                        self.die(GameOverCause::BanditMurder);
                    } else {
                        self.message(format!(
                            "The Harpe brothers fall on you, take {}, and leave you for dead.",
                            fmt_money(loss)
                        ));
                    }
                }
            }
        }
        self.advance();
    }

    /// Crowd-threading route game (keeping to the Trace). `cleared` = the route
    /// was reproduced; `accuracy` (0..1) how far you got before fouling it.
    pub fn resolve_crowd(&mut self, cleared: bool, accuracy: f64) {
        if self.mode != Mode::Crowd {
            return;
        }
        let Some(_) = self.pending_crowd.take() else {
            return;
        };
        if cleared {
            self.adjust_reputation(1.0);
            self.message("You keep the Trace clear in your mind and make good time through the tangle of side trails.");
        } else {
            let drift = (1.0 - accuracy).clamp(0.0, 1.0);
            self.state.miles -= 8.0 + 20.0 * drift;
            self.state.provisions = (self.state.provisions - 6.0).max(0.0);
            self.message("You lose the Trace on a side trail and waste a day doubling back.");
        }
        self.advance();
    }

    /// Timing-bar game. `hit` = struck in the zone; `accuracy` (0..1) how centered.
    pub fn resolve_timing(&mut self, hit: bool, accuracy: f64) {
        if self.mode != Mode::Timing {
            return;
        }
        let Some(task) = self.pending_timing.take() else {
            return;
        };
        match task {
            TimingTask::Gamble => {
                let stake = self.pending_stake;
                self.pending_stake = 0.0;
                if hit {
                    self.state.cash += stake;
                    self.message(format!(
                        "Luck runs your way Under-the-Hill — you walk out {} richer.",
                        fmt_money(stake)
                    ));
                } else {
                    self.state.cash -= stake;
                    if accuracy < 0.3 && self.state.cash > 0.0 {
                        let extra = (self.state.cash * 0.1).floor();
                        self.state.cash -= extra;
                        self.message(format!(
                            "You lose the {} stake — and a cutpurse lifts {} more on your way up the hill.",
                            fmt_money(stake),
                            fmt_money(extra)
                        ));
                    } else {
                        self.message(format!(
                            "The cards turn against you. The {} stake is gone.",
                            fmt_money(stake)
                        ));
                    }
                }
                self.state.cash = self.state.cash.max(0.0);
            }
            TimingTask::Dose => {
                if hit {
                    self.message("You measure the dose right and sweat the fever out by morning.");
                } else {
                    self.state.health -= 10.0;
                    self.message("The fever burns through you — you can barely keep your feet.");
                    if self.state.health <= 0.0 {
                        self.die(GameOverCause::Disease);
                    }
                }
            }
        }
        self.advance();
    }

    /// Sequence (order-memory) game — patching a snagged hull.
    pub fn resolve_sequence(&mut self, correct_prefix: usize, length: usize, perfect: bool) {
        if self.mode != Mode::Sequence {
            return;
        }
        let Some(_) = self.pending_sequence.take() else {
            return;
        };
        let miss = if length == 0 {
            0.0
        } else {
            (1.0 - correct_prefix as f64 / length as f64).clamp(0.0, 1.0)
        };
        if perfect {
            self.message("Wedge, oakum, plank — you patch the hole in good order before much water gets in.");
        } else {
            let lost = self.lose_cargo_fraction(0.05 + 0.15 * miss);
            self.dent_morale(8.0);
            self.message(format!(
                "You fumble the patch and she ships water — {} of cargo is ruined before you stop the leak.",
                fmt_money(lost)
            ));
        }
        self.advance();
    }

    /// Bucket-brigade game — bailing a flooded boat.
    pub fn resolve_brigade(&mut self, contained: bool, leaked: usize, capacity: usize) {
        if self.mode != Mode::Brigade {
            return;
        }
        let Some(_) = self.pending_brigade.take() else {
            return;
        };
        let sev = if capacity > 0 {
            (leaked as f64 / capacity as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if contained {
            let lost = self.lose_cargo_fraction(0.03);
            self.message(format!(
                "You bail her out ahead of the rising water — only {} of cargo lost.",
                fmt_money(lost)
            ));
        } else {
            let lost = self.lose_cargo_fraction(0.08 + 0.25 * sev);
            self.dent_morale(10.0);
            self.message(format!(
                "The flood gets ahead of you — {} of cargo is soaked and lost over the side.",
                fmt_money(lost)
            ));
        }
        self.advance();
    }
}
