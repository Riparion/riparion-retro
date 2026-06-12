//! Fort Nash's outcome vocabulary: a hazard's result expressed as *data* rather
//! than code, mirroring the kaintuck interpreter in
//! [`crate::effect`](../effect/index.html) but with Fort Nash's own resources
//! (food, powder, clothing, supplies, the livestock train) and its own
//! [`EffectTarget`] seam.
//!
//! Like kaintuck's, this is deliberately *not* a scripting language. The only
//! control flow is "select a tier, run its effect list, stop if the party died."
//! Every number is a coefficient; the host computes the severity channel
//! (`drift` = the minigame miss / leak / 1−accuracy) and threads it in, so the
//! interpreter draws no randomness and the seeded golden trace stays stable.

use serde::{Deserialize, Serialize};

pub use crate::effect::Tier;

/// One declarative effect. Coefficients live here; the named reducers that carry
/// them out live on the host's [`EffectTarget`]. Each `Adjust*` is
/// `base + drift_coeff * drift`, so a clean run pays the floor and a botched one
/// the full toll — exactly the `floor + slope * miss` shape of the hand-written
/// resolvers it replaces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Narrate a line, with an optional cover-art key (the slug after the
    /// `interaction-` prefix).
    Message {
        text: String,
        #[serde(default)]
        cover: Option<String>,
    },
    /// Miles delta (lost ground doubling back, or a clean getaway's gain).
    AdjustMiles {
        base: f64,
        #[serde(default)]
        drift_coeff: f64,
    },
    /// Provisions delta.
    AdjustFood {
        base: f64,
        #[serde(default)]
        drift_coeff: f64,
    },
    /// Powder & shot delta (a count of rounds).
    AdjustBullets {
        base: f64,
        #[serde(default)]
        drift_coeff: f64,
    },
    /// Winter clothing & blankets delta.
    AdjustClothing {
        base: f64,
        #[serde(default)]
        drift_coeff: f64,
    },
    /// Miscellaneous supplies / medicine delta.
    AdjustMisc {
        base: f64,
        #[serde(default)]
        drift_coeff: f64,
    },
    /// Livestock-train value delta.
    AdjustOxen {
        base: f64,
        #[serde(default)]
        drift_coeff: f64,
    },
    /// Mark the party as ill (needs tending at the next station).
    SetIll,
    /// Mark the party as injured (needs tending at the next station).
    SetInjured,
    /// End the journey now with this cause key.
    Die(String),
    /// End the journey with this cause key, but only if supplies/medicine
    /// (`misc`) have fallen below zero — the frostbite/pneumonia gate.
    DieIfBroke(String),
}

/// One hazard's outcome, by result tier. The host maps its minigame result to a
/// [`Tier`] (it knows what `steady`/`contained`/`perfect` mean) and applies the
/// matching list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    /// Stable id the host maps its minigame task to.
    pub id: String,
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

/// The per-application context: the severity channel `drift` (the minigame miss,
/// the leak ratio, or 1−accuracy) the host computed for this outcome.
pub struct EffectCtx {
    pub drift: f64,
}

impl EffectCtx {
    pub fn new(drift: f64) -> Self {
        Self { drift }
    }
}

/// The host seam: the named reducers an [`Effect`] is carried out through. These
/// are exactly Fort Nash's existing state mutators, so applying an effect list is
/// indistinguishable from the hand-written resolver arm it replaces.
pub trait EffectTarget {
    fn miles(&mut self, d: f64);
    fn food(&mut self, d: f64);
    fn bullets(&mut self, d: f64);
    fn clothing(&mut self, d: f64);
    fn misc(&mut self, d: f64);
    fn oxen(&mut self, d: f64);
    fn set_ill(&mut self);
    fn set_injured(&mut self);
    /// Current supplies/medicine, for the `DieIfBroke` gate.
    fn misc_now(&self) -> f64;
    /// End the journey with the given cause key.
    fn kill(&mut self, cause_key: &str);
    /// Whether the journey has already ended (stops the effect list).
    fn is_dead(&self) -> bool;
    /// Queue a narrative line with an optional cover key.
    fn narrate(&mut self, text: String, cover: Option<String>);
}

/// Run an effect list against the host. Stops early if a `Die`/`DieIfBroke`
/// ended the journey, so a trailing narration after a fatal blow is correctly
/// skipped.
pub fn apply_effects(target: &mut dyn EffectTarget, effects: &[Effect], ctx: &EffectCtx) {
    for e in effects {
        if target.is_dead() {
            break;
        }
        apply_one(target, e, ctx);
    }
}

fn apply_one(t: &mut dyn EffectTarget, e: &Effect, ctx: &EffectCtx) {
    match e {
        Effect::Message { text, cover } => t.narrate(text.clone(), cover.clone()),
        Effect::AdjustMiles { base, drift_coeff } => t.miles(base + drift_coeff * ctx.drift),
        Effect::AdjustFood { base, drift_coeff } => t.food(base + drift_coeff * ctx.drift),
        Effect::AdjustBullets { base, drift_coeff } => t.bullets(base + drift_coeff * ctx.drift),
        Effect::AdjustClothing { base, drift_coeff } => t.clothing(base + drift_coeff * ctx.drift),
        Effect::AdjustMisc { base, drift_coeff } => t.misc(base + drift_coeff * ctx.drift),
        Effect::AdjustOxen { base, drift_coeff } => t.oxen(base + drift_coeff * ctx.drift),
        Effect::SetIll => t.set_ill(),
        Effect::SetInjured => t.set_injured(),
        Effect::Die(cause) => t.kill(cause),
        Effect::DieIfBroke(cause) => {
            if t.misc_now() < 0.0 {
                t.kill(cause);
            }
        }
    }
}
