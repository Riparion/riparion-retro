//! Trail incidents: war-party encounters, the random-event table, the ridge
//! passes and blizzards, illness, hunting, and the frozen Cumberland crossing.
//! The probability structure and RNG draw order are inherited from the Oregon
//! Trail engine, so seeded play stays reproducible.

use trail_kit::fortnash::{apply_effects, EffectCtx, HazardArm};

use super::interaction::{ShotPurpose, Tactic};
use super::scenario_data::scenario;
use super::state::{EatLevel, GameOverCause};
use super::{BrigadeTask, Flow, Game, Illness, RiderEncounter, SequenceTask};

/// The outcome ids [`Game::begin_event_minigame`] can launch. A `Minigame { outcome }`
/// arm in the scenario's event table may only name one of these; `scenario_is_self_consistent`
/// asserts it, so a typo in the data fails a test instead of panicking mid-game.
/// Keep this in lockstep with the match in `begin_event_minigame`.
#[cfg(test)]
pub(crate) fn is_event_minigame(outcome: &str) -> bool {
    matches!(
        outcome,
        "wheel" | "ox-leg" | "frostbite" | "splint" | "fire" | "rains" | "fog"
    )
}

/// The handler names [`Game::run_special`] knows. A `Special(name)` arm in the
/// event table may only name one of these; `scenario_is_self_consistent` asserts
/// it. Keep this in lockstep with the match in `run_special`.
#[cfg(test)]
pub(crate) fn is_special_handler(name: &str) -> bool {
    matches!(
        name,
        "bad-water" | "outlaws" | "creek" | "wolves" | "sleet" | "illness-roll" | "cold-weather"
    )
}

impl Game {
    // ===== Riders =====

    /// Roll for riders. They're rarer in mid-trail (the original's curve) and
    /// may not be what they appear. Pauses for the player's tactics if they show.
    pub(crate) fn do_riders(&mut self) -> Flow {
        // Encounter chance: appear when rand*10*((u+72)/(u+12)) <= 1, u=(M/100-4)^2.
        let u = (self.state.miles / 100.0 - 4.0).powi(2);
        let mult = 10.0 * (u + 72.0) / (u + 12.0);
        if self.rng.uniform() * mult > 1.0 {
            return Flow::Continue; // no riders this fortnight
        }
        let looks_hostile = self.rng.uniform() < 0.8;
        // 20% of the time the truth differs from the appearance.
        let hostile = if self.rng.uniform() > 0.2 {
            looks_hostile
        } else {
            !looks_hostile
        };
        self.riders = Some(RiderEncounter {
            looks_hostile,
            hostile,
        });
        self.mode = super::Mode::Riders;
        Flow::Pause
    }

    pub(crate) fn resolve_hostile_tactic(&mut self, tactic: Tactic) {
        match tactic {
            Tactic::Run => {
                // Threading away is its own minigame; `resolve_flee` tallies the
                // outcome (clean break vs. run down into a gunfight).
                self.begin_flee();
                return;
            }
            Tactic::Attack => {
                self.begin_shot(ShotPurpose::Riders { circle: false });
                return;
            }
            Tactic::Continue => {
                if self.rng.uniform() > 0.8 {
                    self.message("You press on, and the war party turns off — they did not attack.");
                } else {
                    self.message("The war party harries you as you press on.");
                    self.state.bullets -= 150.0;
                    self.state.misc -= 15.0;
                }
            }
            Tactic::CircleWagons => {
                self.begin_shot(ShotPurpose::Riders { circle: true });
                return;
            }
        }
        self.finish_hostile();
    }

    pub(crate) fn resolve_friendly_tactic(&mut self, tactic: Tactic) {
        match tactic {
            Tactic::Run => {
                self.message("You push hard away from them, tiring the livestock.");
                self.state.miles += 15.0;
                self.state.oxen -= 10.0;
            }
            Tactic::Attack => {
                self.message("You open fire on a friendly band — a waste of powder and time.");
                self.state.miles -= 5.0;
                self.state.bullets -= 100.0;
            }
            Tactic::Continue => {
                self.message("You raise a hand in greeting and pass on by.");
            }
            Tactic::CircleWagons => {
                self.message("You fort up for nothing, losing time.");
                self.state.miles -= 20.0;
            }
        }
        self.message("They were friendly — long hunters, as it turned out. Check the camp for any losses.");
        self.riders = None;
        self.advance();
    }

    /// The gunfight half of an Attack / Circle-wagons against hostile riders.
    pub(crate) fn finish_rider_fight(&mut self, b1: f64, circle: bool) {
        self.state.bullets -= if circle {
            b1 * 30.0 + 80.0
        } else {
            b1 * 40.0 + 80.0
        };
        if circle {
            self.state.miles -= 25.0;
        }
        if b1 <= 1.0 {
            self.message("Nice shooting — you drove them off!");
        } else if b1 > 4.0 {
            self.message("Lousy shot — you took a tomahawk wound. You'll need tending.");
            self.state.injured = true;
        } else {
            self.message("Kinda slow with your longrifle, but you held them off.");
        }
        self.finish_hostile();
    }

    /// Shared tail of every hostile outcome: tally losses, check for a massacre.
    fn finish_hostile(&mut self) {
        self.message("The war party was hostile! Check the camp for losses.");
        self.riders = None;
        if self.state.bullets < 0.0 {
            self.die(GameOverCause::RiderMassacre);
        }
        self.advance();
    }

    // ===== The random-event table =====

    /// Deal one random trail event. A percentile roll selects an arm from the
    /// scenario's event table (`thresholds` are the cumulative percentages from
    /// the BASIC `DATA` list); the arm says, declaratively, which minigame fires,
    /// which constant-toll effects apply, or which RNG-bespoke host handler runs.
    pub(crate) fn do_event(&mut self) -> Flow {
        let r1 = self.rng.uniform() * 100.0;
        let arm = scenario().events.select(r1);
        self.run_hazard_arm(arm)
    }

    /// Carry out a selected event arm.
    pub(crate) fn run_hazard_arm(&mut self, arm: &HazardArm) -> Flow {
        match arm {
            // A constant-toll event (strays, lost child, helpful long-hunters):
            // apply the effect list at zero drift and carry on.
            HazardArm::Effects(effects) => {
                let ctx = EffectCtx::new(0.0);
                apply_effects(self, effects, &ctx);
                Flow::Continue
            }
            // Launch the minigame bound to this outcome; its resolver applies the
            // tier when the player finishes.
            HazardArm::Minigame { outcome } => self.begin_event_minigame(outcome),
            // An RNG-bespoke event keeps its exact draws in a host handler.
            HazardArm::Special(name) => self.run_special(name),
            // Past the mountains, heavy rains become a killing cold snap.
            HazardArm::Branch {
                past_mountains,
                before,
            } => {
                let arm = if self.state.miles > scenario().trail.mountains_at {
                    past_mountains
                } else {
                    before
                };
                self.run_hazard_arm(arm)
            }
        }
    }

    /// Begin the signature minigame an event arm fires, pausing the leg.
    fn begin_event_minigame(&mut self, outcome: &str) -> Flow {
        match outcome {
            "wheel" => self.begin_sequence(SequenceTask::Wheel),
            "ox-leg" => self.begin_sequence(SequenceTask::OxLeg),
            "frostbite" => self.begin_sequence(SequenceTask::Frostbite),
            "splint" => self.begin_splint(),
            "fire" => self.begin_brigade(BrigadeTask::Fire),
            "rains" => self.begin_brigade(BrigadeTask::Rains),
            "fog" => self.begin_fog(),
            // Unreachable for a valid scenario (see `is_event_minigame`, asserted
            // by `scenario_is_self_consistent`); loud in dev if the lists drift.
            other => panic!("unknown event minigame {other}"),
        }
        Flow::Pause
    }

    /// The RNG-bespoke event handlers, kept in the host so their `self.rng` draw
    /// order is exactly the hand-written engine's. The narration is the same, but
    /// the random tolls (and the two gunfight launches, and the illness / cold
    /// rolls) can't be coefficients in the data without perturbing the stream.
    fn run_special(&mut self, name: &str) -> Flow {
        match name {
            "bad-water" => {
                self.message_keyed(
                    "Bad water — you lose time looking for a clean spring.",
                    "unsafe-water",
                );
                self.state.miles -= 10.0 * self.rng.uniform() + 2.0;
                Flow::Continue
            }
            "outlaws" => {
                self.message_keyed("Outlaws block the trace ahead!", "outlaws-attack");
                self.begin_shot(ShotPurpose::Bandits);
                Flow::Pause
            }
            "creek" => {
                // A swollen creek crossing — no signature minigame here (that is
                // saved for the scripted Cumberland ice crossing); just a plain
                // toll of time and a soaking of supplies.
                self.message_keyed(
                    "A swollen creek bars the trace — you ford it cold and lose time and supplies.",
                    "creek-ford",
                );
                self.state.miles -= 6.0 + self.rng.uniform() * 8.0;
                self.state.misc -= 3.0 + self.rng.uniform() * 4.0;
                Flow::Continue
            }
            "wolves" => {
                self.message_keyed("Wolves come out of the timber!", "wolves-attack");
                self.begin_shot(ShotPurpose::WildAnimals);
                Flow::Pause
            }
            "sleet" => {
                self.message_keyed(
                    "A sleet storm batters the column — supplies lost.",
                    "sleet-storm",
                );
                self.state.miles -= 5.0 + self.rng.uniform() * 10.0;
                self.state.bullets -= 200.0;
                self.state.misc -= 4.0 + self.rng.uniform() * 3.0;
                Flow::Continue
            }
            "illness-roll" => {
                // The common "did the way you ate catch up with you?" roll.
                let sick = match self.state.eat_level {
                    EatLevel::Poorly => true,
                    EatLevel::Moderately => self.rng.uniform() > 0.25, // 75%
                    EatLevel::Well => self.rng.uniform() < 0.5,         // 50%
                };
                if sick {
                    // The dosing game runs before the illness is tallied.
                    self.illness()
                } else {
                    Flow::Continue
                }
            }
            "cold-weather" => self.cold_weather(),
            // Unreachable for a valid scenario (see `is_special_handler`, asserted
            // by `scenario_is_self_consistent`); loud in dev if the lists drift.
            other => panic!("unknown event handler {other}"),
        }
    }

    /// Cold-weather check (heavy rains in the mountains): too little clothing
    /// and you fall ill. Pauses for the dosing game if it comes to that.
    fn cold_weather(&mut self) -> Flow {
        let needed = 22.0 + 4.0 * self.rng.uniform();
        if self.state.clothing > needed {
            self.message_keyed(
                "A hard freeze sets in — but you have blankets enough to keep warm.",
                "cold-weather",
            );
            Flow::Continue
        } else {
            self.message_keyed(
                "A hard freeze sets in — and you don't have blankets enough to keep warm!",
                "cold-weather",
            );
            self.illness()
        }
    }

    /// The illness routine: roll the severity (it skews by how well you've been
    /// eating), then put up the dosing minigame. The medicine isn't spent and
    /// the severity isn't applied until [`resolve_dose`](Game::resolve_dose) —
    /// a steady hand wastes less, a shaky one burns extra supplies.
    pub(crate) fn illness(&mut self) -> Flow {
        let e = self.state.eat_level as i64 as f64; // 1 / 2 / 3
        let severity = if self.rng.uniform() * 100.0 < 10.0 + 35.0 * (e - 1.0) {
            Illness::Mild
        } else if self.rng.uniform() * 100.0 < 100.0 - 40.0 / 4f64.powf(e - 1.0) {
            Illness::Bad
        } else {
            Illness::Serious
        };
        self.begin_dose(severity);
        Flow::Pause
    }

    // ===== Mountains =====

    /// The ridge country past [`MOUNTAINS_AT`]. First the rugged going — about a
    /// 30% chance each week — then the one-time passes and the river crossing.
    /// The rugged roll is played out as a route over the ridges via the climb
    /// minigame. When that pauses, [`resolve_climb`](crate::engine::Game::resolve_climb)
    /// falls through to [`do_mountain_passes`] so the passes still run.
    pub(crate) fn do_mountains(&mut self) -> Flow {
        if self.state.miles <= scenario().trail.mountains_at {
            return Flow::Continue;
        }
        // Rough, rugged going — one draw, same as the old quirk's roll, so the
        // RNG stream past here is undisturbed on the (common) no-incident path.
        if self.rng.uniform() < 0.30 {
            self.begin_climb();
            return Flow::Pause;
        }
        self.do_mountain_passes()
    }

    /// Powell Mountain / Wallen's Ridge and, beyond the gap mile, the Cumberland
    /// Gap — each a one-time blizzard gamble — and finally the scripted frozen
    /// Cumberland River crossing. Split out so the climb minigame can run first
    /// and then fall through to here when it resolves.
    pub(crate) fn do_mountain_passes(&mut self) -> Flow {
        // Powell Mountain / Wallen's Ridge — once. (`cleared_south_pass` retains
        // the engine's field name; here it tracks this first hard pass.)
        if !self.state.cleared_south_pass {
            self.state.cleared_south_pass = true;
            if self.rng.uniform() < 0.8 {
                return self.blizzard();
            }
            self.message("You climb over Powell Mountain and Wallen's Ridge — no snow on the ridges yet.");
        }

        // Cumberland Gap — once, past the gap mile.
        if self.state.miles >= scenario().trail.cumberland_gap_at && !self.state.cleared_blue_mountains {
            self.state.cleared_blue_mountains = true;
            if self.rng.uniform() < 0.7 {
                return self.blizzard();
            }
            self.message("You thread the narrow notch of the Cumberland Gap and start down onto the Kentucky plateau.");
        }

        // The frozen Cumberland River — the scripted Christmas crossing, driven
        // over the ice on the steady-hand trace. Fires once, at the river mile.
        if self.state.miles >= scenario().trail.cumberland_river_at && !self.state.cleared_cumberland_river {
            self.state.cleared_cumberland_river = true;
            // The Steady screen narrates the frozen crossing; no queued message
            // (which would otherwise drain after the minigame, out of order).
            self.begin_ice_crossing();
            return Flow::Pause;
        }
        Flow::Continue
    }

    fn blizzard(&mut self) -> Flow {
        // Keeping the fire fed against the wind is its own minigame (the prompt
        // announces the blizzard); `resolve_brigade` tallies the losses graded by
        // how much got away, runs the cold-weather illness check, and falls
        // through to the remaining passes.
        self.begin_brigade(BrigadeTask::Blizzard);
        Flow::Pause
    }

    // ===== Gunfights (resolve the reaction game) =====

    pub(crate) fn finish_bandits(&mut self, b1: f64) {
        self.state.bullets -= 20.0 * b1;
        let out_of_bullets = self.state.bullets < 0.0;
        if out_of_bullets {
            self.message("You ran out of powder — the outlaws make off with two-thirds of your coin!");
            self.state.cash /= 3.0;
        }
        if !out_of_bullets && b1 <= 1.0 {
            self.message("Fastest rifle on the frontier — you drove them off!");
        } else {
            self.message(
                "You took a ball in the leg and they ran off a pack animal. You'll need tending.",
            );
            self.state.injured = true;
            self.state.misc -= 5.0;
            self.state.oxen -= 20.0;
        }
        self.advance();
    }

    pub(crate) fn finish_wolves(&mut self, b1: f64) {
        if self.state.bullets <= 39.0 {
            self.message("You were too low on powder — the wolves overpowered you.");
            self.state.injured = true;
            self.die(GameOverCause::Wolves);
            self.advance();
            return;
        }
        if b1 > 2.0 {
            self.state.bullets -= 20.0 * b1;
            self.message("Slow on the draw — the wolves got at your food and clothes.");
        } else {
            self.message("Nice shootin', partner — the wolves didn't get much.");
        }
        self.state.clothing -= b1 * 4.0;
        self.state.food -= b1 * 8.0;
        self.advance();
    }
}
