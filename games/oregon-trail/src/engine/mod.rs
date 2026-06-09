//! Pure game engine — no UI or platform dependencies. The UI owns a `Game` in
//! a signal and calls these methods; everything here is host-testable.
//!
//! ## Turn flow
//!
//! A fortnight runs: pick an action at the [`Mode::Trail`] hub (hunt / fort /
//! press on) → choose how to [`Mode::Eat`] → travel → then an automated chain
//! of incidents ([`Leg::Riders`] → [`Leg::Event`] → [`Leg::Mountains`]) before
//! the next fortnight. Incidents narrate through the [`Interaction`] queue and
//! can interrupt for a decision (riders' tactics, the [`Mode::Shoot`] reaction
//! game); [`Resume`] and [`Leg`] record where to pick back up when the queue
//! drains, so a refresh resumes exactly in place.

pub mod interaction;
pub use retro_kit::rng;
pub mod scoring;
pub mod state;

mod events;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use interaction::{Interaction, Response, ShotPurpose, Tactic, SHOT_WORDS};
use rng::GameRng;
use state::{
    EatLevel, EndGame, GameState, Mode, BULLETS_PER_DOLLAR, MAX_TURNS, STARTING_CASH, TRAIL_MILES,
};

/// The post-travel chain of trail incidents, run in order each fortnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Leg {
    Riders,
    Event,
    Mountains,
}

/// What to do once the current message queue drains (and no leg is running).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resume {
    Trail,
    Eat,
    Leg,
    NextTurn,
}

/// Whether a trail incident paused for a player decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    Pause,
}

/// Riders on the trail: what they look like, and what they actually are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiderEncounter {
    pub looks_hostile: bool,
    pub hostile: bool,
}

/// The complete game: world state, UI mode, pending messages, and RNG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub state: GameState,
    pub mode: Mode,
    pub pending: VecDeque<Interaction>,
    /// Active incident chain stage, if travel is underway.
    pub leg: Option<Leg>,
    /// Where to resume once the message queue drains.
    pub resume: Resume,
    /// The rider encounter currently being resolved.
    pub riders: Option<RiderEncounter>,
    /// Why the [`Mode::Shoot`] reaction game is up.
    pub shot: Option<ShotPurpose>,
    /// Which of the four words the gunfight is flashing.
    pub shot_word: usize,
    pub outcome: Option<EndGame>,
    pub rng: GameRng,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            state: GameState::new(String::new()),
            mode: Mode::Splash,
            pending: VecDeque::new(),
            leg: None,
            resume: Resume::Trail,
            riders: None,
            shot: None,
            shot_word: 0,
            outcome: None,
            rng: GameRng::from_seed(seed),
        }
    }

    /// Begin outfitting: name the party and self-rate marksmanship (1..=5).
    /// Idempotent under a double-tap — it just re-resets to a fresh outfit.
    pub fn begin(&mut self, party: String, marksman: u8) {
        self.state = GameState::new(party);
        self.state.marksman = marksman.clamp(1, 5);
        self.pending.clear();
        self.leg = None;
        self.riders = None;
        self.shot = None;
        self.outcome = None;
        self.mode = Mode::Outfit;
    }

    /// Spend the $700 grubstake. Dollars on ammunition become 50 bullets each.
    /// Oxen must be $200–$300; nothing may be negative; the lot can't exceed $700.
    pub fn outfit(
        &mut self,
        oxen: f64,
        food: f64,
        ammo_dollars: f64,
        clothing: f64,
        misc: f64,
    ) -> Result<(), String> {
        if self.mode != Mode::Outfit {
            return Ok(());
        }
        if !(200.0..=300.0).contains(&oxen) {
            return Err("Spend $200 to $300 on your oxen team.".into());
        }
        for v in [food, ammo_dollars, clothing, misc] {
            if v < 0.0 {
                return Err("Amounts can't be negative.".into());
            }
        }
        let total = oxen + food + ammo_dollars + clothing + misc;
        if total > STARTING_CASH {
            return Err(format!(
                "That's ${} more than your $700. Spend less.",
                (total - STARTING_CASH) as i64
            ));
        }
        self.state.oxen = oxen;
        self.state.food = food;
        self.state.bullets = ammo_dollars * BULLETS_PER_DOLLAR;
        self.state.clothing = clothing;
        self.state.misc = misc;
        self.state.cash = STARTING_CASH - total;
        // First fortnight starts at turn 0 (March 29) — no time advance yet.
        self.begin_fortnight();
        Ok(())
    }

    // ----- The fortnight hub -----

    /// Stop at the fort this fortnight (costs 45 miles for the detour).
    pub fn choose_fort(&mut self) {
        if self.mode != Mode::Trail || !self.state.fort_available() {
            return;
        }
        self.state.miles -= 45.0;
        self.mode = Mode::Fort;
    }

    /// Buy at the fort: dollars spent on food / ammo / clothing / misc, each
    /// returning two-thirds its value in goods (frontier prices).
    pub fn buy_at_fort(&mut self, food_s: f64, ammo_s: f64, clothing_s: f64, misc_s: f64) {
        if self.mode != Mode::Fort {
            return;
        }
        let spend = [food_s, ammo_s, clothing_s, misc_s].map(|v| v.max(0.0));
        let total: f64 = spend.iter().sum();
        if total <= self.state.cash {
            self.state.food += 2.0 / 3.0 * spend[0];
            self.state.bullets += (2.0 / 3.0 * spend[1] * BULLETS_PER_DOLLAR).floor();
            self.state.clothing += 2.0 / 3.0 * spend[2];
            self.state.misc += 2.0 / 3.0 * spend[3];
            self.state.cash -= total;
        }
        self.goto_eat();
    }

    /// Leave the fort without buying.
    pub fn leave_fort(&mut self) {
        if self.mode != Mode::Fort {
            return;
        }
        self.goto_eat();
    }

    /// Go hunting (costs 45 miles); needs more than 39 bullets.
    pub fn choose_hunt(&mut self) -> Result<(), String> {
        if self.mode != Mode::Trail {
            return Ok(());
        }
        if self.state.bullets <= 39.0 {
            return Err("You need more bullets (40+) to go hunting.".into());
        }
        self.state.miles -= 45.0;
        self.begin_hunt();
        Ok(())
    }

    /// Press on without hunting or stopping.
    pub fn choose_continue(&mut self) {
        if self.mode != Mode::Trail {
            return;
        }
        self.goto_eat();
    }

    /// Eat / starve gate, then the eating screen.
    fn goto_eat(&mut self) {
        if self.state.food < 13.0 {
            self.die("You ran out of food and starved to death.");
        } else {
            self.mode = Mode::Eat;
        }
    }

    /// Eat poorly / moderately / well, then travel.
    pub fn choose_eat(&mut self, level: EatLevel) {
        if self.mode != Mode::Eat {
            return;
        }
        let cost = level.food_cost();
        if self.state.food < cost {
            return; // UI disables unaffordable choices; ignore defensively
        }
        self.state.eat_level = level;
        self.state.food -= cost;
        self.travel();
    }

    /// Add this fortnight's mileage, then launch the incident chain.
    fn travel(&mut self) {
        self.add_fortnight_miles();
        self.leg = Some(Leg::Riders);
        self.resume = Resume::Leg;
        self.advance();
    }

    /// Advance the wagon one fortnight: `M += 200 + (A-220)/5 + 10*rand`.
    /// A stronger oxen team (higher `oxen`) covers more ground.
    pub(crate) fn add_fortnight_miles(&mut self) {
        self.state.miles +=
            200.0 + (self.state.oxen - 220.0) / 5.0 + self.rng.uniform() * 10.0;
    }

    // ----- Hunting gallery -----

    /// Put up the shooting-gallery hunt.
    fn begin_hunt(&mut self) {
        self.mode = Mode::Hunt;
    }

    /// Resolve the hunting gallery. `hit` = the quarry was bagged; `shots_fired`
    /// = rounds spent. Bullets spent track the shots actually fired; a clean kill
    /// on fewer shots feeds the party better. Guarded against stale double-taps
    /// like `resolve_shot`.
    pub fn resolve_hunt(&mut self, hit: bool, shots_fired: usize) {
        if self.mode != Mode::Hunt {
            return;
        }
        self.state.bullets -= shots_fired as f64 * state::BULLETS_PER_SHOT;
        if hit {
            // Fewer shots → bigger haul.
            let bonus = (4u32.saturating_sub(shots_fired as u32)) as f64 * 4.0;
            self.state.food += 48.0 + bonus + self.rng.uniform() * 6.0;
            self.message("Nice shooting — good eatin' tonight!");
        } else {
            self.message("You ran out of rounds, and your dinner got away...");
        }
        self.state.bullets = self.state.bullets.max(0.0);
        self.resume = Resume::Eat;
        self.advance();
    }

    // ----- Outrunning hostile riders -----

    /// Put up the route-memory game: thread the terrain to lose the riders.
    fn begin_flee(&mut self) {
        self.mode = Mode::Flee;
    }

    /// Resolve the escape attempt. `cleared` = the whole route was reproduced
    /// (you lost them); `accuracy` (0..=1) is how far you got before fouling it.
    ///
    /// Running scatters supplies either way — the cleaner your line, the less
    /// you drop. Clear the route and you break away (and gain ground); foul it
    /// and the riders run you down, dropping you into the gunfight. Guarded
    /// against stale double-taps like `resolve_shot`.
    pub fn resolve_flee(&mut self, cleared: bool, accuracy: f64) {
        if self.mode != Mode::Flee {
            return;
        }
        // The sooner you misremember the route, the more you scatter.
        let drop = (1.0 - accuracy).clamp(0.0, 1.0);
        self.state.misc -= 5.0 + 10.0 * drop;
        self.state.oxen -= 10.0 + 30.0 * drop;
        if cleared {
            self.message("You thread the breaks and leave the riders behind — a clean getaway.");
            self.state.miles += 20.0;
            self.riders = None;
            self.advance();
        } else {
            // Caught in the open — the gunfight's own prompt ("Riders close
            // in!") narrates the capture, so no message is queued here.
            self.begin_shot(ShotPurpose::Riders { circle: false });
        }
    }

    // ----- Crossing the rugged mountains -----

    /// Put up the route-memory game: pick a clean line through the rocks.
    fn begin_climb(&mut self) {
        self.mode = Mode::Climb;
    }

    /// Resolve a rugged-mountain crossing. `cleared` = you held the line the
    /// whole way; `accuracy` (0..=1) is how far you got before fouling it. The
    /// original only docked miles (−45..−95); here a clean line barely costs a
    /// step while a fouled one loses the full stretch. Falls through to the
    /// passes (South Pass / Blue Mountains) once the going is tallied. Guarded
    /// against stale double-taps like `resolve_shot`.
    pub fn resolve_climb(&mut self, cleared: bool, accuracy: f64) {
        if self.mode != Mode::Climb {
            return;
        }
        let drop = (1.0 - accuracy).clamp(0.0, 1.0);
        self.state.miles -= 15.0 + 80.0 * drop;
        if cleared {
            self.message("You pick a clean line through the rocks and barely lose a step.");
        } else {
            self.message("The going turns rough — you backtrack through the rocks and lose ground.");
        }
        // Fall through to the passes, then on along the leg to the next fortnight.
        if let Flow::Continue = self.do_mountain_passes() {
            self.advance();
        }
    }

    // ----- Lost in the fog -----

    /// Put up the route-memory game: fix the trail in mind before the fog closes.
    fn begin_fog(&mut self) {
        self.mode = Mode::Fog;
    }

    /// Resolve groping through heavy fog. `cleared` = you held the trail the
    /// whole way; `accuracy` (0..=1) is how far you got before drifting off it.
    /// Keep your bearings and you lose no time; lose your way and you wander —
    /// the original's time penalty, scaled by how far you drifted. Guarded
    /// against stale double-taps like `resolve_shot`.
    pub fn resolve_fog(&mut self, cleared: bool, accuracy: f64) {
        if self.mode != Mode::Fog {
            return;
        }
        if cleared {
            self.message(
                "You keep the trail clear in your mind and come through the fog losing no time.",
            );
        } else {
            let drift = (1.0 - accuracy).clamp(0.0, 1.0);
            self.state.miles -= 5.0 + 10.0 * drift;
            self.message("You lose your way in the fog, wandering before you find the trail again.");
        }
        self.advance();
    }

    // ----- Marksmanship reaction game -----

    /// Put up the gunfight screen with a fresh word to type.
    fn begin_shot(&mut self, purpose: ShotPurpose) {
        self.shot = Some(purpose);
        self.shot_word = self.rng.ri(SHOT_WORDS.len() as i64) as usize;
        self.mode = Mode::Shoot;
    }

    /// Resolve the reaction game. `reaction_secs` is how long the tap took;
    /// `correct` is whether the right word was hit. Yields the original's `B1`
    /// (0 = perfect, larger = slower, 9 = a flubbed word), adjusted by the
    /// marksmanship self-rating, then routed to whatever needed the shot.
    pub fn resolve_shot(&mut self, reaction_secs: f64, correct: bool) {
        if self.mode != Mode::Shoot {
            return;
        }
        let handicap = (self.state.marksman.clamp(1, 5) as f64 - 1.0) * 0.3;
        let b1 = if correct {
            (reaction_secs + handicap).max(0.0)
        } else {
            9.0
        };
        match self.shot.take().unwrap_or(ShotPurpose::Bandits) {
            ShotPurpose::Bandits => self.finish_bandits(b1),
            ShotPurpose::WildAnimals => self.finish_wolves(b1),
            ShotPurpose::Riders { circle } => self.finish_rider_fight(b1, circle),
        }
    }

    // ----- Riders' tactics -----

    /// Resolve the player's choice when riders appear.
    pub fn resolve_tactic(&mut self, tactic: Tactic) {
        if self.mode != Mode::Riders {
            return;
        }
        let enc = match self.riders {
            Some(e) => e,
            None => {
                self.advance();
                return;
            }
        };
        if enc.hostile {
            self.resolve_hostile_tactic(tactic);
        } else {
            self.resolve_friendly_tactic(tactic);
        }
    }

    // ----- Interactions -----

    /// Acknowledge the head message and move the game along. Guarded so a
    /// double-tap on the last message can't double-drain the queue and
    /// advance the turn twice.
    pub fn resolve(&mut self, _response: Response) {
        if self.mode != Mode::Interaction {
            return;
        }
        self.pending.pop_front();
        self.advance();
    }

    pub(crate) fn message(&mut self, text: impl Into<String>) {
        self.pending.push_back(Interaction::Message(text.into()));
    }

    /// Show queued messages, or — if none — resume immediately.
    pub(crate) fn advance(&mut self) {
        if self.pending.is_empty() {
            self.after_queue();
        } else {
            self.mode = Mode::Interaction;
        }
    }

    /// The queue drained: resume whatever was interrupted.
    pub(crate) fn after_queue(&mut self) {
        if self.outcome.is_some() {
            self.mode = Mode::GameOver;
            return;
        }
        match self.resume {
            Resume::Trail => self.mode = Mode::Trail,
            Resume::Eat => self.goto_eat(),
            Resume::Leg => self.run_leg(),
            Resume::NextTurn => self.next_turn(),
        }
    }

    /// Advance the incident chain one stage.
    pub(crate) fn run_leg(&mut self) {
        let flow = match self.leg {
            Some(Leg::Riders) => {
                self.leg = Some(Leg::Event);
                self.do_riders()
            }
            Some(Leg::Event) => {
                self.leg = Some(Leg::Mountains);
                self.do_event()
            }
            Some(Leg::Mountains) => {
                self.leg = None;
                self.resume = Resume::NextTurn;
                self.do_mountains()
            }
            None => {
                self.resume = Resume::NextTurn;
                Flow::Continue
            }
        };
        if let Flow::Continue = flow {
            self.advance();
        }
    }

    // ----- Fortnight boundaries -----

    /// Check arrival, advance the calendar, then set up the new fortnight.
    pub(crate) fn next_turn(&mut self) {
        if self.state.miles >= TRAIL_MILES {
            self.finish_win();
            return;
        }
        self.state.turn += 1;
        if self.state.turn >= MAX_TURNS {
            self.die("You've been on the trail too long. Your family dies in the first blizzard of winter.");
            return;
        }
        self.begin_fortnight();
    }

    /// Per-fortnight opening: tidy supplies, settle the doctor, warn on food.
    pub(crate) fn begin_fortnight(&mut self) {
        self.state.clamp_supplies();
        if self.state.ill || self.state.injured {
            self.state.cash -= 20.0;
            if self.state.cash < 0.0 {
                self.state.cash = 0.0;
                self.die("You couldn't afford a doctor, and your illness took you.");
                return;
            }
            self.message("There is sickness in the wagon. The doctor's bill is $20.");
            self.state.ill = false;
            self.state.injured = false;
        }
        if self.state.food < 13.0 {
            self.message("You'd better do some hunting or buy food — and soon!");
        }
        self.state.miles_at_turn_start = self.state.miles;
        self.leg = None;
        self.resume = Resume::Trail;
        self.advance();
    }

    fn build_end(&self, won: bool, cause: String, arrival: Option<String>, days: i64) -> EndGame {
        let s = &self.state;
        let leftover = scoring::leftover_value(s);
        let miles = s.miles.clamp(0.0, TRAIL_MILES);
        let score = scoring::score(won, miles, days, leftover);
        EndGame {
            won,
            cause,
            arrival,
            miles: miles as i64,
            days,
            food: s.food.max(0.0) as i64,
            bullets: s.bullets.max(0.0) as i64,
            clothing: s.clothing.max(0.0) as i64,
            misc: s.misc.max(0.0) as i64,
            cash: s.cash.max(0.0) as i64,
            score,
            rank: scoring::rank(score).to_string(),
            recorded: false,
        }
    }

    /// End the journey in failure with `cause`.
    pub(crate) fn die(&mut self, cause: impl Into<String>) {
        let days = self.state.turn as i64 * 14;
        self.outcome = Some(self.build_end(false, cause.into(), None, days));
        self.mode = Mode::GameOver;
    }

    /// You made it. Refund the unused slice of the final fortnight's food and
    /// stamp the arrival date.
    pub(crate) fn finish_win(&mut self) {
        let m2 = self.state.miles_at_turn_start;
        let denom = (self.state.miles - m2).max(1.0);
        let f9 = ((TRAIL_MILES - m2) / denom).clamp(0.0, 1.0);
        self.state.food += (1.0 - f9) * self.state.eat_level.food_cost();
        let (arrival, days) = scoring::arrival_date(self.state.turn, f9);
        self.state.miles = TRAIL_MILES;
        let cause =
            "You finally arrived at Oregon City after 2,040 long miles — hooray! A real pioneer!"
                .to_string();
        self.outcome = Some(self.build_end(true, cause, Some(arrival), days));
        self.mode = Mode::GameOver;
    }
}

#[cfg(test)]
mod tests;
