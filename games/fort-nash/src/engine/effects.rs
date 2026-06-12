//! The host seam for the data-driven outcomes: [`Game`] implements
//! [`trail_kit::fortnash::EffectTarget`] so the declarative effect lists in
//! `fort_nash.ron` are carried out through the engine's own state mutators. An
//! effect list is therefore indistinguishable from the hand-written resolver arm
//! it replaces — the same fields move by the same amounts in the same order.

use trail_kit::fortnash::{apply_effects, EffectCtx, EffectTarget, Tier};

use super::scenario_data::scenario;
use super::state::GameOverCause;
use super::Game;

impl Game {
    /// Apply the effect list for `id`'s `tier`, with severity channel `drift`.
    /// The host resolver has already classified the tier (it knows what
    /// `steady`/`contained`/`perfect`/`cleared` mean); this runs the data.
    pub(crate) fn apply_outcome(&mut self, id: &str, tier: Tier, drift: f64) {
        let effects = scenario()
            .outcome(id)
            .unwrap_or_else(|| panic!("missing outcome {id}"))
            .tier(tier);
        let ctx = EffectCtx::new(drift);
        apply_effects(self, effects, &ctx);
    }
}

impl EffectTarget for Game {
    fn miles(&mut self, d: f64) {
        self.state.miles += d;
    }
    fn food(&mut self, d: f64) {
        self.state.food += d;
    }
    fn bullets(&mut self, d: f64) {
        self.state.bullets += d;
    }
    fn clothing(&mut self, d: f64) {
        self.state.clothing += d;
    }
    fn misc(&mut self, d: f64) {
        self.state.misc += d;
    }
    fn oxen(&mut self, d: f64) {
        self.state.oxen += d;
    }
    fn set_ill(&mut self) {
        self.state.ill = true;
    }
    fn set_injured(&mut self) {
        self.state.injured = true;
    }
    fn misc_now(&self) -> f64 {
        self.state.misc
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
}
