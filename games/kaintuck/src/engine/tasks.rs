//! The minigame layer: which catastrophe put up each minigame (held while it
//! runs so the resolve applies the right effect), the `begin_*` launchers, and
//! the `resolve_*` methods the thin UI wrappers route their results into. Every
//! resolve ends by calling [`Game::advance`], which resumes the right leg/voyage
//! chain for the current phase.

use serde::{Deserialize, Serialize};

use trail_kit::effect::{apply_effects, EffectCtx, EffectTarget, Outcome, Tier};

use super::scenario_data::scenario;
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

/// The one minigame currently paused for the player. Holding a single tagged
/// task (rather than six parallel `Option` fields) keeps the live task and the
/// screen it belongs to from ever desyncing, and means a new minigame is one
/// variant plus its [`Self::mode`] arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MiniTask {
    Steady(SteadyTask),
    Quick(QuickTask),
    Crowd(CrowdTask),
    Timing(TimingTask),
    Sequence(SequenceTask),
    Brigade(BrigadeTask),
}

impl MiniTask {
    /// The screen this task is played on.
    pub fn mode(self) -> Mode {
        match self {
            MiniTask::Steady(_) => Mode::Steady,
            MiniTask::Quick(_) => Mode::Quick,
            MiniTask::Crowd(_) => Mode::Crowd,
            MiniTask::Timing(_) => Mode::Timing,
            MiniTask::Sequence(_) => Mode::Sequence,
            MiniTask::Brigade(_) => Mode::Brigade,
        }
    }

    /// The scenario id of this task's outcome and minigame params. The single
    /// task→id map: the resolves, `active_mini_params`, and (inverted)
    /// `begin_minigame_for` all agree through this.
    pub fn outcome_id(self) -> &'static str {
        match self {
            MiniTask::Steady(SteadyTask::Sandbar) => "sandbar",
            MiniTask::Steady(SteadyTask::FallsRun) => "falls-run",
            MiniTask::Steady(SteadyTask::Swamp) => "swamp",
            MiniTask::Steady(SteadyTask::DuckFord) => "duck-ford",
            MiniTask::Quick(QuickTask::Pirates) => "pirates",
            MiniTask::Quick(QuickTask::Mason) => "mason",
            MiniTask::Quick(QuickTask::Harpe) => "harpe",
            MiniTask::Crowd(CrowdTask::SideTrail) => "side-trail",
            MiniTask::Timing(TimingTask::Dose) => "dose",
            MiniTask::Timing(TimingTask::Gamble) => "gamble",
            MiniTask::Sequence(SequenceTask::Patch) => "patch",
            MiniTask::Brigade(BrigadeTask::Bail) => "bail",
        }
    }
}

impl Game {
    // ----- launchers -----

    /// Park the given minigame and switch to its screen.
    fn begin_task(&mut self, task: MiniTask) {
        self.mode = task.mode();
        self.pending_task = Some(task);
    }

    pub(crate) fn begin_steady(&mut self, task: SteadyTask) {
        self.begin_task(MiniTask::Steady(task));
    }
    pub(crate) fn begin_quick(&mut self, task: QuickTask) {
        self.begin_task(MiniTask::Quick(task));
    }
    pub(crate) fn begin_crowd(&mut self, task: CrowdTask) {
        self.begin_task(MiniTask::Crowd(task));
    }
    pub(crate) fn begin_timing(&mut self, task: TimingTask) {
        self.begin_task(MiniTask::Timing(task));
    }
    pub(crate) fn begin_sequence(&mut self, task: SequenceTask) {
        self.begin_task(MiniTask::Sequence(task));
    }
    pub(crate) fn begin_brigade(&mut self, task: BrigadeTask) {
        self.begin_task(MiniTask::Brigade(task));
    }

    /// Begin the minigame whose result selects `outcome` — the bridge from a
    /// data-driven hazard arm (which names an [`Outcome`] id) to the concrete
    /// minigame task. The inverse of [`MiniTask::outcome_id`]; the
    /// `minigame_inverse_map_round_trips` test pins the two in agreement.
    pub(crate) fn begin_minigame_for(&mut self, outcome: &str) {
        match outcome {
            "sandbar" => self.begin_steady(SteadyTask::Sandbar),
            "falls-run" => self.begin_steady(SteadyTask::FallsRun),
            "swamp" => self.begin_steady(SteadyTask::Swamp),
            "duck-ford" => self.begin_steady(SteadyTask::DuckFord),
            "pirates" => self.begin_quick(QuickTask::Pirates),
            "mason" => self.begin_quick(QuickTask::Mason),
            "harpe" => self.begin_quick(QuickTask::Harpe),
            "side-trail" => self.begin_crowd(CrowdTask::SideTrail),
            "dose" => self.begin_timing(TimingTask::Dose),
            "patch" => self.begin_sequence(SequenceTask::Patch),
            "bail" => self.begin_brigade(BrigadeTask::Bail),
            other => panic!("unknown minigame outcome {other}"),
        }
    }

    /// The launch parameters for the minigame currently on screen — the UI reads
    /// these from the scenario instead of hardcoding them per task. Maps the live
    /// `pending_task` to its outcome id, then looks the params up.
    pub fn active_mini_params(&self) -> Option<&'static trail_kit::MiniParams> {
        scenario().minigame_params(self.pending_task?.outcome_id())
    }

    // ----- task accessors (the UI reads the live task to label its screen) -----

    pub fn steady_task(&self) -> Option<SteadyTask> {
        match self.pending_task {
            Some(MiniTask::Steady(t)) => Some(t),
            _ => None,
        }
    }
    pub fn quick_task(&self) -> Option<QuickTask> {
        match self.pending_task {
            Some(MiniTask::Quick(t)) => Some(t),
            _ => None,
        }
    }
    pub fn timing_task(&self) -> Option<TimingTask> {
        match self.pending_task {
            Some(MiniTask::Timing(t)) => Some(t),
            _ => None,
        }
    }

    // ----- resolves -----

    /// Steady-hand trace. `steady` = held on target past the threshold;
    /// `accuracy` (0..1) is the fraction of the run on target.
    pub fn resolve_steady(&mut self, steady: bool, accuracy: f64) {
        if self.mode != Mode::Steady {
            return;
        }
        let Some(MiniTask::Steady(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        let id = MiniTask::Steady(task).outcome_id();
        let o = scenario().outcome(id).expect("missing steady outcome");
        // Quality is the accuracy; holding steady is the success, slipping the
        // partial. A declared catastrophe band can override either.
        let base = if steady { Tier::Success } else { Tier::Partial };
        let tier = resolve_tier(o, base, accuracy);
        let drift = (1.0 - accuracy).clamp(0.0, 1.0);
        self.apply_outcome(id, tier, drift);
        self.advance();
    }

    /// Quick-draw. `reaction_secs` is how long the draw took; `hit` whether the
    /// right target was struck. A clean fast hit is the success; a slow hit is the
    /// partial; a miss is the fail.
    pub fn resolve_quick(&mut self, reaction_secs: f64, hit: bool) {
        if self.mode != Mode::Quick {
            return;
        }
        let Some(MiniTask::Quick(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        let id = MiniTask::Quick(task).outcome_id();
        let o = scenario().outcome(id).expect("missing quick outcome");
        let base = if hit && reaction_secs <= 1.0 {
            Tier::Success
        } else if hit {
            Tier::Partial
        } else {
            Tier::Fail
        };
        // Quality: a clean fast hit is 1, a slow hit 0.5, a miss 0 — so a
        // catastrophe band can make, say, a fumbled draw fatal.
        let quality = if !hit {
            0.0
        } else if reaction_secs <= 1.0 {
            1.0
        } else {
            0.5
        };
        let tier = resolve_tier(o, base, quality);
        self.apply_outcome(id, tier, 0.0);
        self.advance();
    }

    /// Crowd-threading route game (keeping to the Trace). `cleared` = the route
    /// was reproduced; `accuracy` (0..1) how far you got before fouling it.
    pub fn resolve_crowd(&mut self, cleared: bool, accuracy: f64) {
        if self.mode != Mode::Crowd {
            return;
        }
        if !matches!(self.pending_task, Some(MiniTask::Crowd(_))) {
            return;
        }
        self.pending_task = None;
        let o = scenario().outcome("side-trail").expect("missing side-trail outcome");
        let base = if cleared { Tier::Success } else { Tier::Fail };
        let drift = (1.0 - accuracy).clamp(0.0, 1.0);
        let tier = resolve_tier(o, base, accuracy);
        self.apply_outcome("side-trail", tier, drift);
        self.advance();
    }

    /// Timing-bar game. `hit` = struck in the zone; `accuracy` (0..1) how centered.
    pub fn resolve_timing(&mut self, hit: bool, accuracy: f64) {
        if self.mode != Mode::Timing {
            return;
        }
        let Some(MiniTask::Timing(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        match task {
            // The Under-the-Hill gamble stays a hand-coded set-piece (its escrow
            // and cutpurse turn on the live purse); it moves to data with the rest
            // of the Natchez set-piece.
            TimingTask::Gamble => {
                // The stake was escrowed out of cash when the bet was laid
                // (see `gamble`). A win returns it plus equal winnings; a loss
                // simply keeps it.
                let stake = self.pending_stake;
                self.pending_stake = 0.0;
                if hit {
                    self.state.cash += stake * 2.0;
                    self.message(format!(
                        "Luck runs your way Under-the-Hill — you walk out {} richer.",
                        fmt_money(stake)
                    ));
                } else if accuracy < 0.3 && self.state.cash > 0.0 {
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
                self.state.cash = self.state.cash.max(0.0);
            }
            TimingTask::Dose => {
                let o = scenario().outcome("dose").expect("missing dose outcome");
                let base = if hit { Tier::Success } else { Tier::Fail };
                let tier = resolve_tier(o, base, accuracy);
                self.apply_outcome("dose", tier, 0.0);
            }
        }
        self.advance();
    }

    /// Sequence (order-memory) game — patching a snagged hull.
    pub fn resolve_sequence(&mut self, correct_prefix: usize, length: usize, perfect: bool) {
        if self.mode != Mode::Sequence {
            return;
        }
        if !matches!(self.pending_task, Some(MiniTask::Sequence(_))) {
            return;
        }
        self.pending_task = None;
        let o = scenario().outcome("patch").expect("missing patch outcome");
        let miss = if length == 0 {
            0.0
        } else {
            (1.0 - correct_prefix as f64 / length as f64).clamp(0.0, 1.0)
        };
        let base = if perfect { Tier::Success } else { Tier::Partial };
        let tier = resolve_tier(o, base, 1.0 - miss);
        self.apply_outcome("patch", tier, miss);
        self.advance();
    }

    /// Bucket-brigade game — bailing a flooded boat.
    pub fn resolve_brigade(&mut self, contained: bool, leaked: usize, capacity: usize) {
        if self.mode != Mode::Brigade {
            return;
        }
        if !matches!(self.pending_task, Some(MiniTask::Brigade(_))) {
            return;
        }
        self.pending_task = None;
        let o = scenario().outcome("bail").expect("missing bail outcome");
        let sev = if capacity > 0 {
            (leaked as f64 / capacity as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let base = if contained { Tier::Success } else { Tier::Fail };
        let tier = resolve_tier(o, base, 1.0 - sev);
        self.apply_outcome("bail", tier, sev);
        self.advance();
    }

    /// Look up an outcome by id and apply its tier's effect list. The single seam
    /// through which every ported minigame result reaches the world.
    fn apply_outcome(&mut self, id: &str, tier: Tier, drift: f64) {
        let effects = scenario()
            .outcome(id)
            .unwrap_or_else(|| panic!("missing outcome {id}"))
            .tier(tier);
        let mut ctx = EffectCtx::new(drift);
        apply_effects(self, effects, &mut ctx);
    }
}

// ----- outcome lookup glue -----

/// Select the result tier for any minigame. `base` is the kind's ordinary tier
/// (success / partial / fail); `quality` is a normalized 0..1 success metric
/// (higher is better) the host derived from that kind's result. If the outcome
/// declares a `catastrophe_below` band and `quality` falls under it, the
/// catastrophe tier overrides the base — this is how a hazard's minigame is
/// *declaratively* associated with a catastrophe, for every kind, not just the
/// steady-hand set-pieces. A `catastrophe_needs_unsteady` band spares a run that
/// was good enough to land the success tier (the Duck River ford drowns a
/// flounder, not a sure-footed crossing; the Falls wreck any low run) — defined
/// uniformly via `base == Success`, so the refinement works for every kind.
pub(crate) fn resolve_tier(o: &Outcome, base: Tier, quality: f64) -> Tier {
    if let Some(cb) = o.catastrophe_below {
        let clean = base == Tier::Success;
        if quality < cb && (!o.catastrophe_needs_unsteady || !clean) {
            return Tier::Catastrophe;
        }
    }
    base
}

// ----- the host seam the effect interpreter mutates through -----

impl EffectTarget for Game {
    fn lose_cargo(&mut self, frac: f64) -> f64 {
        self.lose_cargo_fraction(frac)
    }
    fn take_cash(&mut self, frac: f64, robbed: bool) -> f64 {
        let loss = (self.state.cash * frac).floor();
        self.state.cash -= loss;
        if robbed {
            self.state.robbed = true;
        }
        loss
    }
    fn morale(&mut self, d: f64) {
        self.state.morale = (self.state.morale + d).max(0.0);
    }
    fn health(&mut self, d: f64) {
        self.state.health += d;
    }
    fn provisions(&mut self, d: f64) {
        self.state.provisions = (self.state.provisions + d).max(0.0);
    }
    fn miles(&mut self, d: f64) {
        self.state.miles += d;
    }
    fn reputation(&mut self, d: f64) {
        self.adjust_reputation(d);
    }
    fn draft(&self) -> f64 {
        self.state.boat.map(|b| b.draft()).unwrap_or(1.0)
    }
    fn grouped(&self) -> bool {
        self.state.grouped
    }
    fn health_now(&self) -> f64 {
        self.state.health
    }
    fn kill(&mut self, cause_key: &str) {
        let cause = GameOverCause::from_key(cause_key)
            .unwrap_or_else(|| panic!("unknown game-over cause {cause_key}"));
        self.die(cause);
    }
    fn is_dead(&self) -> bool {
        self.outcome.is_some()
    }
    fn narrate(&mut self, text: String, cover: Option<String>) {
        match cover {
            Some(c) => self.message_keyed(text, c),
            None => self.message(text),
        }
    }
    fn money(&self, v: f64) -> String {
        fmt_money(v)
    }
}
