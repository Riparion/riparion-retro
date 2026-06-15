//! The minigame layer: which catastrophe put up each minigame (held while it
//! runs so the resolve applies the right effect), the `begin_*` launchers, and
//! the `resolve_*` methods the thin UI wrappers route their results into. Every
//! resolve ends by calling [`Game::advance`], which resumes the right leg/voyage
//! chain for the current phase.

use serde::{Deserialize, Serialize};

use trail_kit::effect::{apply_effects, EffectCtx, EffectTarget, Outcome, Tier};

use super::scenario_data::scenario;
use super::state::{fmt_money, GameOverCause, Mode, Phase};
use super::Game;

/// Which strain put up the steady-hand trace. Steadiness is now only ever
/// "hold a line against a force" — the Falls chute and the Duck River ford.
/// (Grounding moved to [`HeaveTask`], the swamp to [`CrowdTask`].)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SteadyTask {
    /// Running the Falls of the Ohio (River).
    FallsRun,
    /// Running the shoals below Cave-in-Rock (River).
    CaveRun,
    /// Fording the Duck River on foot (Trace).
    DuckFord,
}

/// Which ambush put up the quick-draw. Now the *river* boarding only — the Trace
/// bandits each got their own mechanic (Mason → [`HotColdTask`], Harpe →
/// [`HunterTask`]) so trick, greed, and terror no longer play identically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuickTask {
    /// River pirates pulling alongside to board.
    Pirates,
}

/// Which strain put up the press-and-hold heave. Grounding is exertion, not
/// precision: "all hands overboard to push," handspikes to lever, rollers to
/// roll her off (RESEARCH_NAVIGATION §7a); the cordelle haul at the Grand Chain
/// (§4, §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeaveTask {
    /// Heaving the grounded boat off a sandbar (River).
    Ground,
    /// Hauling the boat through the Grand Chain rapids on the cordelle (River).
    Cordelle,
}

/// Which strain put up the probe-and-deduce search. Mason's man hunts the money
/// you hid — specie sewn in seams, the belt in a hollow log (RESEARCH_PIRATES
/// §5, §7). (The swamp crossing moved to [`CrowdTask`] — pathfinding the firm
/// line through the mud is route-memory, not a blind probe.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotColdTask {
    /// Sam Mason's man searches your camp for your hidden purse (Trace).
    MasonSearch,
}

/// Which strain put up the track-and-shoot. The Harpes "killed for its own sake"
/// (RESEARCH_PIRATES §6) — surrender buys nothing, so you fight; or you hunt game
/// for the pot to spare your provisions (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HunterTask {
    /// A fight for your life against the Harpe brothers, north of the divide.
    Harpe,
    /// Hunting game for the pot on the Trace, to spare the ration.
    Pot,
}

/// Which strain put up the crowd-threading route game: fixing a remembered route
/// in mind, then threading it. Keeping to the Trace among the side trails, or
/// picking the firm line across a swamp's "soupy mud" (RESEARCH_PIRATES §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrowdTask {
    /// Keeping to the Trace among the tangle of side trails.
    SideTrail,
    /// Threading a remembered firm line across a flooded swamp / the "hell
    /// holes" — pathfinding through the soupy mud (Trace).
    Swamp,
}

/// Which strain put up the timing-bar game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimingTask {
    /// A night Under-the-Hill at Natchez.
    Gamble,
    /// Measuring out a dose of medicine against swamp fever.
    Dose,
    /// Reading the fake pilot's tell at Cave-in-Rock before he takes the helm.
    CaveTell,
}

/// Which strain put up the sequence (order-memory) game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SequenceTask {
    /// Patching a snagged hull on the spot, mid-leg (River hazard).
    Patch,
    /// Lying up at a landing to mend the hull yourself — the player-initiated
    /// self-repair, its sequence length scaled to the damage (River).
    SelfRepair,
}

/// Which strain put up the bucket-brigade game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrigadeTask {
    /// Bailing a flooded boat (River).
    Bail,
}

/// Which card table the player sat down at Under-the-Hill. Faro and vingt-et-un
/// were the professionals' games on the Natchez waterfront (RESEARCH_GAMBLING);
/// both are player-initiated set-pieces, never hazard arms, so they have no
/// [`Game::begin_minigame_for`] entry — they're launched straight off the
/// Natchez menu via [`Game::play_faro`] / [`Game::play_vingt_un`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardTask {
    /// A faro bank Under-the-Hill at Natchez.
    Faro,
    /// A hand of vingt-et-un (twenty-one) at a Natchez table.
    VingtUn,
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
    Heave(HeaveTask),
    HotCold(HotColdTask),
    Hunter(HunterTask),
    Card(CardTask),
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
            MiniTask::Heave(_) => Mode::Heave,
            MiniTask::HotCold(_) => Mode::HotCold,
            MiniTask::Hunter(_) => Mode::Hunter,
            MiniTask::Card(CardTask::Faro) => Mode::Faro,
            MiniTask::Card(CardTask::VingtUn) => Mode::VingtUn,
        }
    }

    /// The scenario id of this task's outcome and minigame params. The single
    /// task→id map: the resolves, `active_mini_params`, and (inverted)
    /// `begin_minigame_for` all agree through this.
    pub fn outcome_id(self) -> &'static str {
        match self {
            MiniTask::Steady(SteadyTask::FallsRun) => "falls-run",
            MiniTask::Steady(SteadyTask::CaveRun) => "cave-run",
            MiniTask::Steady(SteadyTask::DuckFord) => "duck-ford",
            MiniTask::Quick(QuickTask::Pirates) => "pirates",
            MiniTask::Heave(HeaveTask::Ground) => "sandbar",
            MiniTask::Heave(HeaveTask::Cordelle) => "cordelle",
            MiniTask::HotCold(HotColdTask::MasonSearch) => "mason",
            MiniTask::Hunter(HunterTask::Harpe) => "harpe",
            MiniTask::Hunter(HunterTask::Pot) => "trace-hunt",
            MiniTask::Crowd(CrowdTask::SideTrail) => "side-trail",
            MiniTask::Crowd(CrowdTask::Swamp) => "swamp",
            MiniTask::Timing(TimingTask::Dose) => "dose",
            MiniTask::Timing(TimingTask::Gamble) => "gamble",
            MiniTask::Timing(TimingTask::CaveTell) => "cave-tell",
            MiniTask::Sequence(SequenceTask::Patch) => "patch",
            MiniTask::Sequence(SequenceTask::SelfRepair) => "self-repair",
            MiniTask::Brigade(BrigadeTask::Bail) => "bail",
            MiniTask::Card(CardTask::Faro) => "faro",
            MiniTask::Card(CardTask::VingtUn) => "vingt-un",
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
    pub(crate) fn begin_heave(&mut self, task: HeaveTask) {
        self.begin_task(MiniTask::Heave(task));
    }
    pub(crate) fn begin_hotcold(&mut self, task: HotColdTask) {
        self.begin_task(MiniTask::HotCold(task));
    }
    pub(crate) fn begin_hunter(&mut self, task: HunterTask) {
        self.begin_task(MiniTask::Hunter(task));
    }
    pub(crate) fn begin_card(&mut self, task: CardTask) {
        self.begin_task(MiniTask::Card(task));
    }

    /// Begin the minigame whose result selects `outcome` — the bridge from a
    /// data-driven hazard arm (which names an [`Outcome`] id) to the concrete
    /// minigame task. The inverse of [`MiniTask::outcome_id`]; the
    /// `minigame_inverse_map_round_trips` test pins the two in agreement.
    pub(crate) fn begin_minigame_for(&mut self, outcome: &str) {
        match outcome {
            "falls-run" => self.begin_steady(SteadyTask::FallsRun),
            "cave-run" => self.begin_steady(SteadyTask::CaveRun),
            "duck-ford" => self.begin_steady(SteadyTask::DuckFord),
            "pirates" => self.begin_quick(QuickTask::Pirates),
            "sandbar" => self.begin_heave(HeaveTask::Ground),
            "cordelle" => self.begin_heave(HeaveTask::Cordelle),
            "swamp" => self.begin_crowd(CrowdTask::Swamp),
            "mason" => self.begin_hotcold(HotColdTask::MasonSearch),
            "harpe" => self.begin_hunter(HunterTask::Harpe),
            "trace-hunt" => self.begin_hunter(HunterTask::Pot),
            "side-trail" => self.begin_crowd(CrowdTask::SideTrail),
            "dose" => self.begin_timing(TimingTask::Dose),
            "patch" => self.begin_sequence(SequenceTask::Patch),
            "self-repair" => self.begin_sequence(SequenceTask::SelfRepair),
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
    pub fn crowd_task(&self) -> Option<CrowdTask> {
        match self.pending_task {
            Some(MiniTask::Crowd(t)) => Some(t),
            _ => None,
        }
    }
    pub fn timing_task(&self) -> Option<TimingTask> {
        match self.pending_task {
            Some(MiniTask::Timing(t)) => Some(t),
            _ => None,
        }
    }
    pub fn heave_task(&self) -> Option<HeaveTask> {
        match self.pending_task {
            Some(MiniTask::Heave(t)) => Some(t),
            _ => None,
        }
    }
    pub fn hotcold_task(&self) -> Option<HotColdTask> {
        match self.pending_task {
            Some(MiniTask::HotCold(t)) => Some(t),
            _ => None,
        }
    }
    pub fn hunter_task(&self) -> Option<HunterTask> {
        match self.pending_task {
            Some(MiniTask::Hunter(t)) => Some(t),
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

    /// Crowd-threading route game — keeping to the Trace among side trails, or
    /// threading the firm line across a swamp from memory. `cleared` = the route
    /// was reproduced; `accuracy` (0..1) how far you got before fouling it.
    pub fn resolve_crowd(&mut self, cleared: bool, accuracy: f64) {
        if self.mode != Mode::Crowd {
            return;
        }
        let Some(MiniTask::Crowd(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        let id = MiniTask::Crowd(task).outcome_id();
        let o = scenario().outcome(id).expect("missing crowd outcome");
        let base = if cleared { Tier::Success } else { Tier::Fail };
        let drift = (1.0 - accuracy).clamp(0.0, 1.0);
        let tier = resolve_tier(o, base, accuracy);
        self.apply_outcome(id, tier, drift);
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
            // Reading the fake pilot at Cave-in-Rock. Hand-coded like the gamble:
            // its branch turns on the live cargo value (the wreckers only bothered
            // to ground a hold worth the betrayal — RESEARCH_PIRATES §4), so a clean
            // read saves you, a misread on a fat hold drops you into the boarding.
            TimingTask::CaveTell => {
                if hit {
                    self.adjust_reputation(2.0);
                    self.message("Something in the pilot's patter rings false. You wave him off and thread the shoals on the Navigator's own marks — clean, and you kept your dollar.");
                } else if self.state.cargo_value() > 60.0 {
                    // You missed the tell with a hold worth taking — you've tangled
                    // with Mason's river gang at his own lair, and it follows you
                    // onto the Trace as the wanted-poster payoff.
                    self.state.crossed_mason = true;
                    self.message("You miss the tell. The stranger takes your steering oar — and runs you straight onto the bar below the cave, where his friends are already pushing off in skiffs.");
                    self.begin_quick(QuickTask::Pirates);
                    // begin_quick set the screen; don't fall through to advance.
                    return;
                } else {
                    self.message("You misjudge the man, but your thin cargo isn't worth the betrayal. He pilots you through honestly enough — luck, more than judgment.");
                }
            }
        }
        self.advance();
    }

    /// Sequence (order-memory) game — patching a snagged hull mid-leg, or the
    /// player-initiated self-repair lying up at a landing.
    pub fn resolve_sequence(&mut self, correct_prefix: usize, length: usize, perfect: bool) {
        if self.mode != Mode::Sequence {
            return;
        }
        let Some(MiniTask::Sequence(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        let frac = if length == 0 {
            0.0
        } else {
            (correct_prefix as f64 / length as f64).clamp(0.0, 1.0)
        };
        let miss = 1.0 - frac;
        match task {
            SequenceTask::Patch => {
                let o = scenario().outcome("patch").expect("missing patch outcome");
                let base = if perfect { Tier::Success } else { Tier::Partial };
                let tier = resolve_tier(o, base, frac);
                self.apply_outcome("patch", tier, miss);
            }
            SequenceTask::SelfRepair => self.resolve_self_repair(correct_prefix, frac, perfect),
        }
        self.advance();
    }

    /// Apply a self-repair attempt to the hull: a flawless run mends her wholly,
    /// a slip mends in proportion to how far you got, and a botched run (no step
    /// right) leaves her worse than before. The damage math is hand-coded (it
    /// mutates the boat's condition, not the standard effect channels); the
    /// `self-repair` outcome supplies the per-tier narration and morale flavor.
    fn resolve_self_repair(&mut self, correct_prefix: usize, frac: f64, perfect: bool) {
        let bomb_penalty = scenario().repair.bomb_penalty;
        let tier = if perfect {
            Tier::Success
        } else if correct_prefix == 0 {
            Tier::Fail
        } else {
            Tier::Partial
        };
        // Narrate first, so a botch that wrecks her (bomb past 100) still tells
        // the tale before the game-over.
        self.apply_outcome("self-repair", tier, 0.0);
        // Then move the hull through the shared chokepoint: a flawless run mends
        // her whole, a slip mends in proportion, a botch leaves her worse (and
        // can sink her, like any other damage path).
        let cur = self.state.boat_damage();
        let delta = match tier {
            Tier::Success => -cur,
            Tier::Partial => -(cur * frac),
            _ => bomb_penalty,
        };
        self.adjust_boat_damage(delta);
    }

    /// The self-repair sequence length, scaled to the current hull damage: from
    /// the minigame's own `length` (a sound-ish hull) up to `self_seq_max_len` (a
    /// wreck). `None` for any other sequence, so the host falls back to the
    /// scenario length. Owning the task-type branch here keeps the UI dumb.
    pub fn self_repair_seq_len(&self) -> Option<usize> {
        if !matches!(
            self.pending_task,
            Some(MiniTask::Sequence(SequenceTask::SelfRepair))
        ) {
            return None;
        }
        let base = match scenario().minigame_params("self-repair") {
            Some(trail_kit::MiniParams::Sequence { length, .. }) => *length,
            _ => return None,
        };
        let max = scenario().repair.self_seq_max_len.max(base);
        let bump = ((max - base) as f64 * self.state.boat_damage() / 100.0).round() as usize;
        Some(base + bump)
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

    /// Heave (press-and-hold exertion) — heaving a grounded boat off a bar, or
    /// hauling on the cordelle. `opened` = every resistance stage gave way;
    /// `slips` = over-exertions; `grip_left` (0..1) = crew strength left at the
    /// end. Clearing her is the success; running out of grip the partial.
    pub fn resolve_heave(&mut self, opened: bool, slips: usize, grip_left: f64) {
        if self.mode != Mode::Heave {
            return;
        }
        let Some(MiniTask::Heave(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        let id = MiniTask::Heave(task).outcome_id();
        let o = scenario().outcome(id).expect("missing heave outcome");
        let base = if opened { Tier::Success } else { Tier::Partial };
        // Quality is the grip you had to spare, docked for each slipped handspike;
        // a band can make an utterly botched heave (snapped poles, no grip) worse.
        let quality = (grip_left - 0.15 * slips as f64).clamp(0.0, 1.0);
        // Drift scales the cargo-overboard loss: the more you flounder, the more
        // washes off the bar.
        let drift = (1.0 - grip_left).clamp(0.0, 1.0);
        let tier = resolve_tier(o, base, quality);
        self.apply_outcome(id, tier, drift);
        self.advance();
    }

    /// HotCold (probe-and-deduce search). `found` = the target was located;
    /// `probes_used` vs. the search `budget` (the encounter's `max_probes`) grades
    /// how sharp it was. We grade against the *budget*, not the kit's `par`:
    /// kaintuck deliberately caps `max_probes` below the grid's natural par (a
    /// tight search), so within that budget "good play" is finding it in the first
    /// half, not under the lenient full-grid par (grading on par leaves the slow
    /// tier unreachable, since `probes_used` never reaches it). Ambushed, *you*
    /// scramble to reach your hidden purse before Mason's men do: fast = clean
    /// getaway, slow = they grab a share, never = they take the lot.
    pub fn resolve_hotcold(&mut self, found: bool, probes_used: usize, budget: usize) {
        if self.mode != Mode::HotCold {
            return;
        }
        let Some(MiniTask::HotCold(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        let id = MiniTask::HotCold(task).outcome_id();
        let o = scenario().outcome(id).expect("missing hotcold outcome");
        let budget = budget.max(1);
        // A find in the first half is "sharp".
        let sharp = probes_used * 2 <= budget;
        let (base, quality, drift) = match task {
            HotColdTask::MasonSearch => {
                // Ambushed, you scramble in the dark to reach your hidden purse
                // before Mason's men do. Reaching it sharply is a clean getaway
                // (you palm them a decoy); reaching it slowly, they grab a share;
                // never reaching it, they take it all and leave you bleeding.
                let base = if !found {
                    Tier::Fail
                } else if sharp {
                    Tier::Success
                } else {
                    Tier::Partial
                };
                (base, if found { 1.0 } else { 0.0 }, 0.0)
            }
        };
        let tier = resolve_tier(o, base, quality);
        self.apply_outcome(id, tier, drift);
        self.advance();
    }

    /// Hunter (track-and-shoot, finite ammo) — the Harpe fight, or hunting for the
    /// pot. `hit` = the quarry was struck; `shots_fired` = rounds spent. A clean
    /// one- or two-shot kill is the success; a kill that emptied the gun the
    /// partial; an empty gun the fail.
    pub fn resolve_hunter(&mut self, hit: bool, shots_fired: usize) {
        if self.mode != Mode::Hunter {
            return;
        }
        let Some(MiniTask::Hunter(task)) = self.pending_task else {
            return;
        };
        self.pending_task = None;
        let id = MiniTask::Hunter(task).outcome_id();
        let o = scenario().outcome(id).expect("missing hunter outcome");
        let base = if hit && shots_fired <= 2 {
            Tier::Success
        } else if hit {
            Tier::Partial
        } else {
            Tier::Fail
        };
        // Quality: a clean kill is 1, a ragged one 0.5, a miss 0. It only bites if
        // an outcome declares a `catastrophe_below` band; neither Hunter outcome
        // does today, so it's an inert forward hook — the Harpes' lethality comes
        // from the Fail tier's own AdjustHealth(-20) + DieIfDead, not from quality.
        let quality = if !hit {
            0.0
        } else if shots_fired <= 2 {
            1.0
        } else {
            0.5
        };
        let tier = resolve_tier(o, base, quality);
        self.apply_outcome(id, tier, 0.0);
        self.advance();
    }

    /// Settle a faro session Under-the-Hill. Hand-coded like the timing gamble
    /// (its economy turns on the live purse, not the tiered effect tables): the
    /// buy-in was escrowed out of cash when the player sat down (see
    /// [`Self::play_faro`]), so all that's left is to hand back the chips they
    /// rose with and tell the tale. `final_stake` is those chips; `won` (reached
    /// the table's mark) only colors the telling and a small reputation bump.
    pub fn resolve_faro(&mut self, won: bool, final_stake: i64) {
        if self.mode != Mode::Faro {
            return;
        }
        if !matches!(self.pending_task, Some(MiniTask::Card(CardTask::Faro))) {
            return;
        }
        self.pending_task = None;
        self.settle_card_night("faro bank", won, final_stake);
        self.advance();
    }

    /// Settle a vingt-et-un session — the twin of [`Self::resolve_faro`].
    pub fn resolve_vingt_un(&mut self, won: bool, final_stake: i64) {
        if self.mode != Mode::VingtUn {
            return;
        }
        if !matches!(self.pending_task, Some(MiniTask::Card(CardTask::VingtUn))) {
            return;
        }
        self.pending_task = None;
        self.settle_card_night("vingt-et-un table", won, final_stake);
        self.advance();
    }

    /// Return the chips a card session ended on to the purse and narrate the
    /// swing against the escrowed buy-in. Shared by faro and vingt-et-un so the
    /// two settle identically — only the table's name and the flavor differ.
    fn settle_card_night(&mut self, table: &str, won: bool, final_stake: i64) {
        let buy_in = self.pending_stake;
        self.pending_stake = 0.0;
        let chips = final_stake.max(0) as f64;
        self.state.cash = (self.state.cash + chips).max(0.0);
        let swing = chips - buy_in;
        if swing > 0.0 {
            // A clean win (you reached the mark) reads as skill at the table and
            // is worth a little standing; merely walking up is just luck.
            self.adjust_reputation(if won { 3.0 } else { 1.0 });
            self.message(format!(
                "You rise from the {table} {} to the good — and climb the hill with a fuller purse.",
                fmt_money(swing)
            ));
        } else if swing < 0.0 {
            self.message(format!(
                "The {table} takes you for {} before the night is out.",
                fmt_money(-swing)
            ));
        } else {
            self.message(format!(
                "You break even at the {table} — a night's amusement for nothing gained."
            ));
        }
    }

    /// Look up an outcome by id and apply its tier's effect list. The single seam
    /// through which every ported minigame result reaches the world. On the river,
    /// a battered hull adds to the severity `drift`, so a damaged boat bleeds more
    /// cargo on every mishap (the `drift_coeff` channel) — see [`RepairParams`].
    fn apply_outcome(&mut self, id: &str, tier: Tier, drift: f64) {
        let effects = scenario()
            .outcome(id)
            .unwrap_or_else(|| panic!("missing outcome {id}"))
            .tier(tier);
        let drift = drift + self.hull_drift();
        let mut ctx = EffectCtx::new(drift);
        apply_effects(self, effects, &mut ctx);
    }

    /// Extra severity drift a damaged hull adds to a river hazard's outcome — the
    /// "worse outcomes when battered" tooth. Zero off the water or with no boat.
    fn hull_drift(&self) -> f64 {
        if self.phase != Phase::River {
            return 0.0;
        }
        scenario().repair.outcome_drift_coeff * self.state.boat_damage() / 100.0
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
    fn boat_damage(&mut self, d: f64) {
        self.adjust_boat_damage(d);
    }
    fn draft(&self) -> f64 {
        self.state.boat.map(|b| b.draft()).unwrap_or(1.0)
    }
    fn grouped(&self) -> bool {
        // Either kind of company counts: `grouped` on the Trace, `river_convoy`
        // on the water. They never overlap in practice — `grouped` is only ever
        // set in the Trace hub, and `enter_trace` clears `river_convoy` on the way
        // in — so this also feeds the pirates' `halve_if_grouped` cargo relief.
        self.state.grouped || self.state.river_convoy
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
