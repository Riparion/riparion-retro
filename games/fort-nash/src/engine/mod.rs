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
pub mod scenario_data;
pub mod scoring;
pub mod state;

mod effects;
mod events;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use trail_kit::fortnash::{MinigameParams, Tier};

use interaction::{Interaction, Response, ShotPurpose, Tactic, SHOT_WORDS};
use rng::GameRng;
use scenario_data::scenario;
use state::{EatLevel, EndGame, GameOverCause, GameState, Mode};

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

/// How bad the illness turned out to be. Rolled when the sickness strikes, then
/// held while the dosing minigame runs so the resolve applies the right effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Illness {
    Mild,
    Bad,
    Serious,
}

/// Which catastrophe put up the sequence (order-memory) minigame, held while it
/// runs so the resolve applies the right losses. Each trades reproducing a short
/// ordered procedure for graded supply hits — get the order right and you pay the
/// floor, fumble it and you pay the full toll (and, for the frostbite, the
/// medicine that keeps you alive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SequenceTask {
    /// Cart wheel breaks — jack, block, bolt, and re-seat it in order.
    Wheel,
    /// A pack animal hurt its leg — wrap, pad, bind in sequence.
    OxLeg,
    /// Frostbite — work the first-aid steps in order (warm → wrap → bind).
    Frostbite,
}

/// Which catastrophe put up the bucket-brigade minigame, held while it runs so
/// the resolve applies the right losses. All three trade a triage-against-spread
/// race for graded supply hits — contain the threat and you pay the floor, let
/// it run and you pay the full toll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrigadeTask {
    /// Fire in the wagon — stamp out the flames before they reach the supplies.
    Fire,
    /// Heavy rains — bail and cover the load as the leaks spread.
    Rains,
    /// Blizzard in the pass — keep the fire fed against the wind.
    Blizzard,
}

/// The one minigame currently paused for the player. Holding a single tagged task
/// (rather than four parallel `Option` fields) keeps the live task and the screen
/// it belongs to from ever desyncing, and means a new minigame is one variant plus
/// its [`Self::mode`] arm. The shot/riders encounters carry their own state and
/// stay separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MiniTask {
    /// Measuring out a dose for an illness of the rolled severity.
    Dose(Illness),
    /// Driving the livestock over the frozen Cumberland (the steady-hand trace).
    Ice,
    /// Reproducing an ordered first-aid/repair procedure.
    Sequence(SequenceTask),
    /// Beating back a spreading threat (fire, sleet, blizzard).
    Brigade(BrigadeTask),
}

impl MiniTask {
    /// The screen this task is played on.
    pub fn mode(self) -> Mode {
        match self {
            MiniTask::Dose(_) => Mode::Dose,
            MiniTask::Ice => Mode::Steady,
            MiniTask::Sequence(_) => Mode::Sequence,
            MiniTask::Brigade(_) => Mode::Brigade,
        }
    }
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
    /// The one minigame (dose / ice / sequence / brigade) currently paused, if any.
    pub pending_task: Option<MiniTask>,
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
            pending_task: None,
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
        self.resume = Resume::Trail;
        self.riders = None;
        self.shot = None;
        self.shot_word = 0;
        self.pending_task = None;
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
        let start = &scenario().start;
        if !(start.oxen_min..=start.oxen_max).contains(&oxen) {
            return Err("Spend $200 to $300 on your livestock.".into());
        }
        for v in [food, ammo_dollars, clothing, misc] {
            if v < 0.0 {
                return Err("Amounts can't be negative.".into());
            }
        }
        let total = oxen + food + ammo_dollars + clothing + misc;
        if total > start.cash {
            return Err(format!(
                "That's ${} more than your $700. Spend less.",
                (total - start.cash) as i64
            ));
        }
        self.state.oxen = oxen;
        self.state.food = food;
        self.state.bullets = ammo_dollars * start.bullets_per_dollar;
        self.state.clothing = clothing;
        self.state.misc = misc;
        self.state.cash = start.cash - total;
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
        self.state.miles -= scenario().fort.detour_miles;
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
            let fort = &scenario().fort;
            // Frontier prices: goods return value_num/value_den of their cost.
            let value = fort.value_num / fort.value_den;
            let bpd = scenario().start.bullets_per_dollar;
            self.state.food += value * spend[0];
            self.state.bullets += (value * spend[1] * bpd).floor();
            self.state.clothing += value * spend[2];
            self.state.misc += value * spend[3];
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
        if self.state.bullets <= scenario().fort.hunt_min_bullets {
            return Err("You need more powder & shot (40+ rounds) to go hunting.".into());
        }
        self.state.miles -= scenario().fort.detour_miles;
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
        if self.state.food < EatLevel::Poorly.food_cost() {
            self.die(GameOverCause::Starved);
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
        self.state.bullets -= shots_fired as f64 * scenario().start.bullets_per_shot;
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
        if cleared {
            self.apply_outcome("flee", Tier::Success, drop);
            self.riders = None;
            self.advance();
        } else {
            // Caught in the open — the gunfight's own prompt ("Riders close
            // in!") narrates the capture, so no message is queued here.
            self.apply_outcome("flee", Tier::Fail, drop);
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
        let tier = if cleared { Tier::Success } else { Tier::Fail };
        self.apply_outcome("climb", tier, drop);
        // Fall through to the passes, then on along the leg to the next fortnight.
        self.resume_after_mountain_incident();
    }

    /// After a mountain-stage minigame (climb / blizzard) resolves, run the
    /// remaining one-time passes; if they don't pause for another minigame, drain
    /// the queue and carry on along the leg. Shared by `resolve_climb` and the
    /// blizzard arm of `resolve_brigade` so the fall-through lives in one place.
    fn resume_after_mountain_incident(&mut self) {
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
        let drift = (1.0 - accuracy).clamp(0.0, 1.0);
        let tier = if cleared { Tier::Success } else { Tier::Fail };
        self.apply_outcome("fog", tier, drift);
        self.advance();
    }

    // ----- Setting a broken bone -----

    /// Put up the timing game: set the broken bone with one clean strike.
    fn begin_splint(&mut self) {
        self.mode = Mode::Splint;
    }

    /// Resolve splinting the daughter's arm. `set_clean` = the strike landed in
    /// the zone; `accuracy` (0..=1) is how dead-center it was. A clean set barely
    /// costs time; a botched one loses ground and burns through supplies — the
    /// original's `−5..−9` miles / `−2..−5` misc, graded by the hand's steadiness.
    /// Guarded against stale double-taps like `resolve_shot`.
    pub fn resolve_splint(&mut self, set_clean: bool, accuracy: f64) {
        if self.mode != Mode::Splint {
            return;
        }
        let drop = (1.0 - accuracy).clamp(0.0, 1.0);
        let tier = if set_clean { Tier::Success } else { Tier::Fail };
        self.apply_outcome("splint", tier, drop);
        self.advance();
    }

    // ----- Measuring out medicine -----

    /// Park the given minigame and switch to its screen.
    fn begin_task(&mut self, task: MiniTask) {
        self.mode = task.mode();
        self.pending_task = Some(task);
    }

    // ----- task accessors (the UI reads the live task to label its screen) -----

    pub fn illness_task(&self) -> Option<Illness> {
        match self.pending_task {
            Some(MiniTask::Dose(i)) => Some(i),
            _ => None,
        }
    }
    pub fn sequence_task(&self) -> Option<SequenceTask> {
        match self.pending_task {
            Some(MiniTask::Sequence(t)) => Some(t),
            _ => None,
        }
    }
    pub fn brigade_task(&self) -> Option<BrigadeTask> {
        match self.pending_task {
            Some(MiniTask::Brigade(t)) => Some(t),
            _ => None,
        }
    }
    /// Whether the paused minigame is the frozen-Cumberland ice crossing.
    pub fn is_ice_crossing(&self) -> bool {
        matches!(self.pending_task, Some(MiniTask::Ice))
    }

    /// The scenario id of the minigame currently on screen, mapping the live mode
    /// and pending task to its entry. `None` for non-minigame modes and for the
    /// gunfight/hunt, which keep their own screens.
    pub fn active_minigame_id(&self) -> Option<&'static str> {
        Some(match self.mode {
            Mode::Steady => "ice",
            Mode::Splint => "splint",
            Mode::Fog => "fog",
            Mode::Flee => "flee",
            Mode::Climb => "climb",
            Mode::Sequence => match self.sequence_task()? {
                SequenceTask::Wheel => "wheel",
                SequenceTask::OxLeg => "ox-leg",
                SequenceTask::Frostbite => "frostbite",
            },
            Mode::Brigade => match self.brigade_task()? {
                BrigadeTask::Fire => "fire",
                BrigadeTask::Rains => "rains",
                BrigadeTask::Blizzard => "blizzard",
            },
            Mode::Dose => match self.illness_task()? {
                Illness::Mild => "dose-mild",
                Illness::Bad => "dose-bad",
                Illness::Serious => "dose-serious",
            },
            _ => return None,
        })
    }

    /// The launch parameters of the minigame currently on screen, read from the
    /// scenario, for the generic minigame host.
    pub fn active_mini_params(&self) -> Option<&'static MinigameParams> {
        scenario_data::scenario().minigame_params(self.active_minigame_id()?)
    }

    /// Escape hatch for the generic minigame host: bail out of a minigame whose
    /// params can't be rendered (a desynced save, or a scenario the consistency
    /// test would reject) as a no-cost clean pass, rather than stranding the
    /// player on a blank, undismissable screen. Applies no toll; just resumes the
    /// interrupted flow the way a resolved minigame does.
    pub fn skip_unrenderable_minigame(&mut self) {
        self.advance();
    }

    /// Put up the timing game: measure out a dose for the illness just rolled.
    fn begin_dose(&mut self, severity: Illness) {
        self.begin_task(MiniTask::Dose(severity));
    }

    /// Resolve measuring out the medicine. `on_target` = the pour landed in the
    /// zone; `accuracy` (0..=1) is how close to the mark it was. The illness's
    /// own toll (the original's per-severity miles/misc) lands either way; a
    /// shaky pour spills extra medical supplies on top, up to a full dose's
    /// worth — which can be what tips a thin party into pneumonia. Guarded
    /// against stale double-taps like `resolve_shot`.
    pub fn resolve_dose(&mut self, on_target: bool, accuracy: f64) {
        if self.mode != Mode::Dose {
            return;
        }
        let Some(MiniTask::Dose(severity)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        // The illness's own toll, the spill (graded by how shaky the pour was, the
        // `drift` channel), and the pneumonia gate if the medicine runs out all live
        // in the outcome's tiers; a steady pour is Success, a shaky one Fail.
        let id = match severity {
            Illness::Mild => "dose-mild",
            Illness::Bad => "dose-bad",
            Illness::Serious => "dose-serious",
        };
        let waste = (1.0 - accuracy).clamp(0.0, 1.0);
        let tier = if on_target { Tier::Success } else { Tier::Fail };
        self.apply_outcome(id, tier, waste);
        self.advance();
    }

    // ----- Holding a hand steady under strain -----

    /// Put up the precision-trace game for the frozen Cumberland ice crossing.
    fn begin_ice_crossing(&mut self) {
        self.begin_task(MiniTask::Ice);
    }

    /// Resolve the steady-hand trace over the frozen Cumberland. `steady` = the run
    /// held on target past the pass threshold; `accuracy` (0..=1) is the fraction of
    /// the run on target. The losses are graded by how shaky the hand was: a steady
    /// run costs the floor, a wandering one the full toll, and a badly shaky crossing
    /// breaks the ice. Guarded against stale double-taps like `resolve_dose`.
    pub fn resolve_steady(&mut self, steady: bool, accuracy: f64) {
        if self.mode != Mode::Steady {
            return;
        }
        if !matches!(self.pending_task, Some(MiniTask::Ice)) {
            return;
        }
        self.pending_task = None;
        // How much the hand drifted: 0 = dead steady, 1 = all over the place. A
        // badly shaky crossing is the Catastrophe tier — the ice cracks and takes
        // the party down (the splinter line drains before GameOver like every
        // other death-with-narration path).
        let drift = (1.0 - accuracy).clamp(0.0, 1.0);
        let tier = if !steady && drift > 0.6 {
            Tier::Catastrophe
        } else if steady {
            Tier::Success
        } else {
            Tier::Fail
        };
        self.apply_outcome("ice", tier, drift);
        self.advance();
    }

    // ----- Reproducing an ordered procedure from memory -----

    /// Put up the order-memory game for whichever catastrophe just struck.
    fn begin_sequence(&mut self, task: SequenceTask) {
        self.begin_task(MiniTask::Sequence(task));
    }

    /// Resolve the order-memory game. `correct_prefix` of `length` steps were
    /// reproduced in order before the first slip; `perfect` flags a flawless run.
    /// Each task's losses are graded by how much of the procedure was fumbled —
    /// get the order right and you pay the floor, botch it and you pay the full
    /// toll (the snakebite still fatal if it spills the last of the medicine).
    /// Guarded against stale double-taps like `resolve_steady`.
    pub fn resolve_sequence(&mut self, correct_prefix: usize, length: usize, perfect: bool) {
        if self.mode != Mode::Sequence {
            return;
        }
        let Some(MiniTask::Sequence(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        // How much of the procedure was botched: 0 = flawless order, 1 = fumbled
        // from the very first step. Each task's graded toll (and the frostbite's
        // death-if-no-medicine gate) lives in its outcome's tiers; a flawless run
        // is Success, any slip is Fail.
        let miss = if length == 0 {
            0.0
        } else {
            (1.0 - correct_prefix as f64 / length as f64).clamp(0.0, 1.0)
        };
        let id = match task {
            SequenceTask::Wheel => "wheel",
            SequenceTask::OxLeg => "ox-leg",
            SequenceTask::Frostbite => "frostbite",
        };
        let tier = if perfect { Tier::Success } else { Tier::Fail };
        self.apply_outcome(id, tier, miss);
        self.advance();
    }

    // ----- Beating back a spreading threat -----

    /// A per-encounter PRNG seed derived from current progress (fortnight +
    /// mileage) — varies between encounters yet stays deterministic for a given
    /// save, and never draws from (so never perturbs) the game's own RNG stream.
    /// `salt` keeps co-located minigames distinct so a same-fortnight splint, dose,
    /// and steady trace don't share a layout. The thin minigame screen wrappers all
    /// derive their `seed` through this one helper.
    pub fn encounter_seed(&self, salt: u64) -> u64 {
        (self.state.turn as u64)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ self.state.miles.to_bits()
            ^ salt
    }

    /// Put up the triage-against-spread game for whichever catastrophe just struck.
    fn begin_brigade(&mut self, task: BrigadeTask) {
        self.begin_task(MiniTask::Brigade(task));
    }

    /// Resolve the bucket-brigade. `contained` = the threat was beaten back to
    /// zero before the clock ran out; `leaked`/`capacity` are the live-cell count
    /// at the buzzer and the grid size, which grade the loss: contain it (`leaked`
    /// 0) and you pay the floor, let it spread across the whole grid (`leaked` ≈
    /// `capacity`) and you pay the full toll. Normalizing here — rather than in the
    /// UI — keeps the grading unit-tested. The blizzard additionally runs the
    /// cold-weather illness check and falls through to the remaining passes,
    /// mirroring `resolve_climb`. Guarded against stale double-taps like
    /// `resolve_steady`.
    pub fn resolve_brigade(&mut self, contained: bool, leaked: usize, capacity: usize) {
        if self.mode != Mode::Brigade {
            return;
        }
        let Some(MiniTask::Brigade(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        // How much got away: 0 = beaten back clean, 1 = it spread across the load.
        // The per-task graded toll and the two outcome lines live in the outcome's
        // tiers; containing it is Success, letting it run is Fail.
        let sev = if capacity > 0 {
            (leaked as f64 / capacity as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let tier = if contained { Tier::Success } else { Tier::Fail };
        let id = match task {
            BrigadeTask::Fire => "fire",
            BrigadeTask::Rains => "rains",
            BrigadeTask::Blizzard => "blizzard",
        };
        self.apply_outcome(id, tier, sev);
        match task {
            // Fire and Rains are a plain triage — the toll's applied, so carry on.
            BrigadeTask::Fire | BrigadeTask::Rains => self.advance(),
            BrigadeTask::Blizzard => {
                // Too little clothing and the cold brings on sickness (the dosing
                // game), exactly as the original blizzard did.
                if self.state.clothing < 18.0 + 2.0 * self.rng.uniform() {
                    self.illness();
                    return;
                }
                // Otherwise fall through to the remaining one-time passes (the
                // Cumberland Gap and the river crossing) and on along the leg.
                self.resume_after_mountain_incident();
            }
        }
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
        // No-op on a missing purpose (a stale double-tap / desynced save) rather
        // than fabricating a gunfight, matching the other resolvers' guard.
        let Some(purpose) = self.shot.take() else {
            return;
        };
        let handicap = (self.state.marksman.clamp(1, 5) as f64 - 1.0) * 0.3;
        let b1 = if correct {
            (reaction_secs + handicap).max(0.0)
        } else {
            9.0
        };
        match purpose {
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
        self.pending.push_back(Interaction::Message {
            text: text.into(),
            cover: None,
        });
    }

    /// Queue a message that also carries a narrative cover-art key (the slug
    /// after the `interaction-` prefix; see FORTNASH_IMAGE_KEYS.md).
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
        if self.state.miles >= scenario().trail.total_miles {
            self.finish_win();
            return;
        }
        self.state.turn += 1;
        if self.state.turn >= scenario().trail.max_turns {
            self.die(GameOverCause::Winter);
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
                self.die(GameOverCause::CantAffordDoctor);
                return;
            }
            self.message("There is sickness in the camp. Tending the sick costs you $20 in trade goods.");
            self.state.ill = false;
            self.state.injured = false;
        }
        if self.state.food < EatLevel::Poorly.food_cost() {
            self.message("You'd better do some hunting or find food — and soon!");
        }
        self.state.miles_at_turn_start = self.state.miles;
        self.leg = None;
        self.resume = Resume::Trail;
        self.advance();
    }

    fn build_end(&self, cause: GameOverCause, arrival: Option<String>, days: i64) -> EndGame {
        let s = &self.state;
        let won = cause.won();
        let leftover = scoring::leftover_value(s);
        let miles = s.miles.clamp(0.0, scenario().trail.total_miles);
        let score = scoring::score(won, miles, days, leftover);
        EndGame {
            won,
            cause: cause.message().to_string(),
            cause_kind: cause,
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
    pub(crate) fn die(&mut self, cause: GameOverCause) {
        let days = self.state.turn as i64 * 7;
        self.outcome = Some(self.build_end(cause, None, days));
        self.mode = Mode::GameOver;
    }

    /// You made it. Refund the unused slice of the final fortnight's food and
    /// stamp the arrival date.
    pub(crate) fn finish_win(&mut self) {
        let m2 = self.state.miles_at_turn_start;
        let denom = (self.state.miles - m2).max(1.0);
        let total_miles = scenario().trail.total_miles;
        let f9 = ((total_miles - m2) / denom).clamp(0.0, 1.0);
        self.state.food += (1.0 - f9) * self.state.eat_level.food_cost();
        let (arrival, days) = scoring::arrival_date(self.state.turn, f9);
        self.state.miles = total_miles;
        self.outcome = Some(self.build_end(GameOverCause::Victory, Some(arrival), days));
        self.mode = Mode::GameOver;
    }
}

#[cfg(test)]
mod tests;
