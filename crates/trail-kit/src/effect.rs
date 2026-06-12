//! The outcome vocabulary: how a hazard's result is described as *data* rather
//! than code. An [`Outcome`] holds, per result [`Tier`], a small ordered list of
//! [`Effect`]s. The host applies them through the [`EffectTarget`] trait — a thin
//! seam over the game's own state mutators — so this crate stays game-agnostic
//! while the actual purse, hold, and health live in the host.
//!
//! This is deliberately *not* a scripting language. The only control flow is
//! "select a tier, run its effect list, stop if the traveller died." Every
//! number is a coefficient; every branch is a tier. That is expressive enough to
//! capture the hand-written outcomes exactly, and no more.

use serde::{Deserialize, Serialize};

/// Which band a minigame result fell into. The host computes this from the
/// minigame's own result fields (it knows what "steady" or "a slow draw" mean);
/// the data only supplies the effect list per band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Success,
    Partial,
    Fail,
    Catastrophe,
}

/// One declarative effect. Coefficients live here; the named reducers that carry
/// them out live on the host's [`EffectTarget`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Narrate a line, with an optional cover-art key. The text may contain the
    /// tokens `{cargo_lost}` and `{loss}`, filled from the values produced by a
    /// preceding `LoseCargoFraction` / `TakeCashFraction` in the same list.
    Message {
        text: String,
        #[serde(default)]
        cover: Option<String>,
    },
    /// Lose `base + drift_coeff*drift` of the cargo, optionally scaled by the
    /// boat's draft and/or halved when travelling in company.
    LoseCargoFraction {
        base: f64,
        #[serde(default)]
        drift_coeff: f64,
        #[serde(default)]
        scale_by_draft: bool,
        #[serde(default)]
        halve_if_grouped: bool,
    },
    /// Crew morale delta.
    AdjustMorale(f64),
    /// Health delta (unclamped — pair with `DieIfDead` to end on a fatal blow).
    AdjustHealth(f64),
    /// Provisions delta, floored at zero.
    AdjustProvisions(f64),
    /// Miles delta, `base + drift_coeff*drift` (lost ground doubling back).
    AdjustMiles {
        base: f64,
        #[serde(default)]
        drift_coeff: f64,
    },
    /// Reputation delta (the host clamps it).
    AdjustReputation(f64),
    /// Take `frac` (or `frac_grouped` when in company) of the purse.
    TakeCashFraction {
        frac: f64,
        frac_grouped: f64,
        #[serde(default)]
        set_robbed: bool,
    },
    /// End the journey now with this cause key.
    Die(String),
    /// End the journey with this cause key, but only if health has fallen to zero.
    DieIfDead(String),
}

/// One hazard's outcome, by result tier, plus the steady-hand catastrophe line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    /// Stable id the host maps its minigame task to.
    pub id: String,
    /// For steady-hand set-pieces: an accuracy below this is a [`Tier::Catastrophe`].
    #[serde(default)]
    pub catastrophe_below: Option<f64>,
    /// Whether the catastrophe also requires an unsteady run (the Duck River ford
    /// drowns only a *floundering* crossing; the Falls wreck any low run).
    #[serde(default)]
    pub catastrophe_needs_unsteady: bool,
    #[serde(default)]
    pub success: Vec<Effect>,
    #[serde(default)]
    pub partial: Vec<Effect>,
    #[serde(default)]
    pub fail: Vec<Effect>,
    #[serde(default)]
    pub catastrophe: Vec<Effect>,
}

impl Outcome {
    /// The effect list for a tier.
    pub fn tier(&self, t: Tier) -> &[Effect] {
        match t {
            Tier::Success => &self.success,
            Tier::Partial => &self.partial,
            Tier::Fail => &self.fail,
            Tier::Catastrophe => &self.catastrophe,
        }
    }
}

/// The per-application context: the severity channel `drift` (1−accuracy, or the
/// sequence miss, or the brigade leak ratio), and the values produced mid-list
/// for message interpolation.
pub struct EffectCtx {
    pub drift: f64,
    cargo_lost: f64,
    cash_loss: f64,
}

impl EffectCtx {
    pub fn new(drift: f64) -> Self {
        Self {
            drift,
            cargo_lost: 0.0,
            cash_loss: 0.0,
        }
    }
}

/// The host seam: the named reducers an [`Effect`] is carried out through. These
/// are exactly the game's existing state mutators, so applying an effect list is
/// indistinguishable from the hand-written arm it replaces.
pub trait EffectTarget {
    /// Lose `frac` of the cargo; returns the market value lost (for narration).
    fn lose_cargo(&mut self, frac: f64) -> f64;
    /// Take `frac` of the purse; returns the amount lost. `robbed` marks the
    /// traveller as having been robbed on the Trace.
    fn take_cash(&mut self, frac: f64, robbed: bool) -> f64;
    fn morale(&mut self, d: f64);
    fn health(&mut self, d: f64);
    fn provisions(&mut self, d: f64);
    fn miles(&mut self, d: f64);
    fn reputation(&mut self, d: f64);
    /// The boat's draft (1.0 if none), for draft-scaled cargo loss.
    fn draft(&self) -> f64;
    /// Whether travelling in company.
    fn grouped(&self) -> bool;
    /// Current health, for `DieIfDead`.
    fn health_now(&self) -> f64;
    /// End the journey with the given cause key.
    fn kill(&mut self, cause_key: &str);
    /// Whether the journey has already ended (stops the effect list).
    fn is_dead(&self) -> bool;
    /// Queue a narrative line with an optional cover key.
    fn narrate(&mut self, text: String, cover: Option<String>);
    /// Format a dollar value the way the game does.
    fn money(&self, v: f64) -> String;
}

/// Run an effect list against the host. Stops early if a `Die`/`DieIfDead` ended
/// the journey, so a trailing narration after a fatal blow is correctly skipped.
pub fn apply_effects(target: &mut dyn EffectTarget, effects: &[Effect], ctx: &mut EffectCtx) {
    for e in effects {
        if target.is_dead() {
            break;
        }
        apply_one(target, e, ctx);
    }
}

fn apply_one(t: &mut dyn EffectTarget, e: &Effect, ctx: &mut EffectCtx) {
    match e {
        Effect::Message { text, cover } => {
            let text = substitute(t, ctx, text);
            t.narrate(text, cover.clone());
        }
        Effect::LoseCargoFraction {
            base,
            drift_coeff,
            scale_by_draft,
            halve_if_grouped,
        } => {
            let mut frac = base + drift_coeff * ctx.drift;
            if *scale_by_draft {
                frac *= t.draft();
            }
            if *halve_if_grouped && t.grouped() {
                frac /= 2.0;
            }
            ctx.cargo_lost = t.lose_cargo(frac);
        }
        Effect::AdjustMorale(d) => t.morale(*d),
        Effect::AdjustHealth(d) => t.health(*d),
        Effect::AdjustProvisions(d) => t.provisions(*d),
        Effect::AdjustMiles { base, drift_coeff } => t.miles(base + drift_coeff * ctx.drift),
        Effect::AdjustReputation(d) => t.reputation(*d),
        Effect::TakeCashFraction {
            frac,
            frac_grouped,
            set_robbed,
        } => {
            let f = if t.grouped() { *frac_grouped } else { *frac };
            ctx.cash_loss = t.take_cash(f, *set_robbed);
        }
        Effect::Die(cause) => t.kill(cause),
        Effect::DieIfDead(cause) => {
            if t.health_now() <= 0.0 {
                t.kill(cause);
            }
        }
    }
}

/// Fill the `{cargo_lost}` / `{loss}` tokens from the values produced so far.
fn substitute(t: &dyn EffectTarget, ctx: &EffectCtx, text: &str) -> String {
    let mut s = text.to_string();
    if s.contains("{cargo_lost}") {
        s = s.replace("{cargo_lost}", &t.money(ctx.cargo_lost));
    }
    if s.contains("{loss}") {
        s = s.replace("{loss}", &t.money(ctx.cash_loss));
    }
    s
}
