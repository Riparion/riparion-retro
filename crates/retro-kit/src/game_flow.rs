//! Persistence glue shared by every game's root component. The save-on-every-
//! transition / record-the-score-on-game-over effect is identical across games
//! once you abstract over each game's own `Mode`, storage keys, and high-score
//! type. A game implements [`Persisted`] on its `Game` and drops
//! [`use_persistence`] into its root; the rest of the root (screen dispatch,
//! splash logic) stays game-specific.

use dioxus::prelude::*;

/// The persistence contract a game's root provides. All the game-specific bits
/// — which modes are transient, where saves live, how a score is recorded —
/// live in the implementation; the Dioxus effect that drives them lives here.
pub trait Persisted: Clone + PartialEq + 'static {
    /// Pre-journey screens (e.g. Splash / NewGame) that should not be persisted.
    fn is_transient(&self) -> bool;
    /// The terminal game-over screen.
    fn is_game_over(&self) -> bool;
    /// Persist the in-progress game to localStorage.
    fn save(&self);
    /// A journey has ended: clear the save and, if the outcome has not yet been
    /// recorded, post its score to the hall of fame and mark it recorded.
    fn finish(game: Signal<Self>);
}

/// Persist every transition; finished journeys clear the save and post a score.
/// Drop this into the game's root component: `use_persistence(game);`.
pub fn use_persistence<G: Persisted>(game: Signal<G>) {
    use_effect(move || {
        let snapshot = game.read();
        if snapshot.is_transient() {
            return;
        }
        if snapshot.is_game_over() {
            drop(snapshot);
            G::finish(game);
        } else {
            snapshot.save();
        }
    });
}
