pub mod components;
pub mod screens;

/// Which full-screen overlay sits above the gameplay modes. Provided as a single
/// `Signal<Overlay>` so the two title-screen toggles don't collide (Dioxus keys
/// context by type, so two `Signal<bool>` contexts would alias each other).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// The title screen (every load opens here).
    Splash,
    /// The "How to Rule" rules screen.
    Help,
    /// No overlay — the current [`crate::engine::state::Mode`] shows through.
    Game,
}
