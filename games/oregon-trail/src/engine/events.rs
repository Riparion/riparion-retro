//! Trail incidents: the rider encounters, the random-event table, the mountain
//! passes and blizzards, illness, hunting, and gunfights. Every probability and
//! formula is transcribed from the 1975 MECC BASIC (see the spec in the port
//! plan); the RNG draw order is preserved so seeded play is reproducible.

use super::interaction::{ShotPurpose, Tactic};
use super::state::{EatLevel, GameOverCause, BLUE_MOUNTAINS_AT, MOUNTAINS_AT};
use super::{BrigadeTask, Flow, Game, Illness, RiderEncounter, SteadyTask};

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
                    self.message("You keep riding, and the riders turn off — they did not attack.");
                } else {
                    self.message("The riders harry you as you press on.");
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
                self.message("You ride hard away from them, tiring the oxen.");
                self.state.miles += 15.0;
                self.state.oxen -= 10.0;
            }
            Tactic::Attack => {
                self.message("You open fire on a friendly band — a waste of bullets and time.");
                self.state.miles -= 5.0;
                self.state.bullets -= 100.0;
            }
            Tactic::Continue => {
                self.message("You wave and ride on by.");
            }
            Tactic::CircleWagons => {
                self.message("You circle the wagons for nothing, losing time.");
                self.state.miles -= 20.0;
            }
        }
        self.message("The riders were friendly. Check the wagon for any losses.");
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
            self.message("Lousy shot — you got knifed. You'll need to see the doctor.");
            self.state.injured = true;
        } else {
            self.message("Kinda slow with your Colt .45, but you held them off.");
        }
        self.finish_hostile();
    }

    /// Shared tail of every hostile outcome: tally losses, check for a massacre.
    fn finish_hostile(&mut self) {
        self.message("The riders were hostile! Check the wagon for losses.");
        self.riders = None;
        if self.state.bullets < 0.0 {
            self.die(GameOverCause::RiderMassacre);
        }
        self.advance();
    }

    // ===== The random-event table =====

    /// Deal one random trail event. Thresholds are cumulative percentages from
    /// the BASIC `DATA` list; the first one `r1` falls under selects the event.
    pub(crate) fn do_event(&mut self) -> Flow {
        let r1 = self.rng.uniform() * 100.0;
        const THRESH: [f64; 15] = [
            6.0, 11.0, 13.0, 15.0, 17.0, 22.0, 32.0, 35.0, 37.0, 42.0, 44.0, 54.0, 64.0, 69.0, 95.0,
        ];
        let mut idx = 16usize; // r1 > 95 → helpful Indians
        for (i, t) in THRESH.iter().enumerate() {
            if r1 <= *t {
                idx = i + 1;
                break;
            }
        }
        match idx {
            1 => {
                self.message_keyed(
                    "One of your wagon wheels breaks down. Lose time and supplies.",
                    "wagon-wheel-breaks",
                );
                self.state.miles -= 15.0 + 5.0 * self.rng.uniform();
                self.state.misc -= 8.0;
            }
            2 => {
                // Steadying the hurt leg to wrap it is its own minigame;
                // `resolve_steady` tallies how snug the wrap held.
                self.begin_steady(SteadyTask::OxLeg);
                return Flow::Pause;
            }
            3 => {
                // Setting the bone cleanly is its own minigame; `resolve_splint`
                // tallies how steady the hand was.
                self.begin_splint();
                return Flow::Pause;
            }
            4 => {
                self.message_keyed(
                    "An ox wanders off. You spend time rounding it up.",
                    "ox-wanders-off",
                );
                self.state.miles -= 17.0;
            }
            5 => {
                self.message_keyed(
                    "Your son gets lost. You hunt for him and lose time.",
                    "son-lost",
                );
                self.state.miles -= 10.0;
            }
            6 => {
                self.message_keyed(
                    "Unsafe water — you lose time looking for a clean spring.",
                    "unsafe-water",
                );
                self.state.miles -= 10.0 * self.rng.uniform() + 2.0;
            }
            7 => {
                if self.state.miles > MOUNTAINS_AT {
                    // Falling ill from the cold pauses for the dosing game.
                    return self.cold_weather();
                }
                // Bailing and covering the load as the leaks spread is its own
                // minigame (the prompt announces the downpour); `resolve_brigade`
                // tallies how much of the load got soaked.
                self.begin_brigade(BrigadeTask::Rains);
                return Flow::Pause;
            }
            8 => {
                self.message_keyed("Bandits attack!", "bandits-attack");
                self.begin_shot(ShotPurpose::Bandits);
                return Flow::Pause;
            }
            9 => {
                // Stamping out the flames before they reach the supplies is its
                // own minigame (the prompt announces the fire); `resolve_brigade`
                // tallies what still burned when the smoke cleared.
                self.begin_brigade(BrigadeTask::Fire);
                return Flow::Pause;
            }
            10 => {
                // Heavy fog — finding the trail through it is its own minigame;
                // `resolve_fog` tallies whether you kept your bearings.
                self.begin_fog();
                return Flow::Pause;
            }
            11 => {
                // Drawing the venom with a steady blade is its own minigame
                // (the prompt announces the bite); `resolve_steady` tallies how
                // much medicine the hand wasted — and whether enough was left to
                // survive.
                self.begin_steady(SteadyTask::Snakebite);
                return Flow::Pause;
            }
            12 => {
                // Holding the wagon level in the current is its own minigame
                // (the prompt announces the ford); `resolve_steady` tallies how
                // much of the load washed away.
                self.begin_steady(SteadyTask::Ford);
                return Flow::Pause;
            }
            13 => {
                self.message_keyed("Wild animals attack!", "wild-animals-attack");
                self.begin_shot(ShotPurpose::WildAnimals);
                return Flow::Pause;
            }
            14 => {
                self.message_keyed(
                    "A hail storm batters the wagon — supplies lost.",
                    "hail-storm",
                );
                self.state.miles -= 5.0 + self.rng.uniform() * 10.0;
                self.state.bullets -= 200.0;
                self.state.misc -= 4.0 + self.rng.uniform() * 3.0;
            }
            15 => {
                // The common "did the way you ate catch up with you?" roll.
                let sick = match self.state.eat_level {
                    EatLevel::Poorly => true,
                    EatLevel::Moderately => self.rng.uniform() > 0.25, // 75%
                    EatLevel::Well => self.rng.uniform() < 0.5,         // 50%
                };
                if sick {
                    // The dosing game runs before the illness is tallied.
                    return self.illness();
                }
            }
            _ => {
                self.message_keyed(
                    "Helpful Indians show you where to find more food.",
                    "helpful-indians",
                );
                self.state.food += 14.0;
            }
        }
        Flow::Continue
    }

    /// Cold-weather check (heavy rains in the mountains): too little clothing
    /// and you fall ill. Pauses for the dosing game if it comes to that.
    fn cold_weather(&mut self) -> Flow {
        let needed = 22.0 + 4.0 * self.rng.uniform();
        if self.state.clothing > needed {
            self.message_keyed(
                "Cold weather ahead — but you have enough clothing to stay warm.",
                "cold-weather",
            );
            Flow::Continue
        } else {
            self.message_keyed(
                "Cold weather ahead — and you don't have enough clothing to keep warm!",
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

    /// The high country past mile 950. First the rugged going — about a 30%
    /// chance each fortnight — then the passes. The original's rugged roll was a
    /// dead quirk (it could only fire on an exact-zero draw); here it's a real
    /// incident, played out as a route through the rocks via the climb minigame.
    /// When that pauses, [`resolve_climb`](crate::engine::Game::resolve_climb)
    /// falls through to [`do_mountain_passes`] so the passes still run.
    pub(crate) fn do_mountains(&mut self) -> Flow {
        if self.state.miles <= MOUNTAINS_AT {
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

    /// South Pass and, beyond mile 1700, the Blue Mountains — each a one-time
    /// blizzard gamble. Split out so the climb minigame can run first and then
    /// fall through to here when it resolves.
    pub(crate) fn do_mountain_passes(&mut self) -> Flow {
        // South Pass — once.
        if !self.state.cleared_south_pass {
            self.state.cleared_south_pass = true;
            if self.rng.uniform() < 0.8 {
                return self.blizzard();
            }
            self.message("You made it safely through the South Pass — no snow.");
        }

        // Blue Mountains — once, past mile 1700.
        if self.state.miles >= BLUE_MOUNTAINS_AT && !self.state.cleared_blue_mountains {
            self.state.cleared_blue_mountains = true;
            if self.rng.uniform() < 0.7 {
                return self.blizzard();
            }
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
            self.message("You ran out of bullets — the bandits make off with two-thirds of your cash!");
            self.state.cash /= 3.0;
        }
        if !out_of_bullets && b1 <= 1.0 {
            self.message("Quickest draw outside of Dodge City! You got 'em!");
        } else {
            self.message(
                "You got shot in the leg and they took one of your oxen. Better see a doctor.",
            );
            self.state.injured = true;
            self.state.misc -= 5.0;
            self.state.oxen -= 20.0;
        }
        self.advance();
    }

    pub(crate) fn finish_wolves(&mut self, b1: f64) {
        if self.state.bullets <= 39.0 {
            self.message("You were too low on bullets — the wolves overpowered you.");
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
