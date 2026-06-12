//! Set-piece menus as data: the Falls, Natchez Under-the-Hill, and the trace
//! stands are each a panel of buttons, and a button is just a label, a hint, a
//! cost, a visibility gate, and an action tag the host dispatches. Keeping the
//! menu *structure* here — options, ordering, costs, conditions — lets the host
//! render every set-piece through one generic component, and a scenario reorder
//! or reprice a menu without code.
//!
//! [`SetPiece`] and [`SetPieceOption`] are generic over the host game's gate
//! vocabulary `G`, so both kaintuck (the [`Gate`] below) and Fort Nash
//! ([`fortnash::Gate`](crate::fortnash::Gate)) share one button/menu shape and
//! supply only their own visibility conditions — exactly as both share [`Tier`].
//!
//! [`Tier`]: crate::Tier

use serde::{Deserialize, Serialize};

/// The set-piece menus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Menus {
    pub falls: SetPiece<Gate>,
    /// Grand Tower, on the Mississippi: the rivermen's initiation — stand a
    /// treat or take the ducking. Defaults to empty so other scenarios that
    /// never reach it can omit it.
    #[serde(default)]
    pub grandtower: SetPiece<Gate>,
    pub natchez: SetPiece<Gate>,
    pub stand: SetPiece<Gate>,
}

/// One set-piece: an ordered list of options gated by the host game's `G`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetPiece<G> {
    pub options: Vec<SetPieceOption<G>>,
}

// Hand-written so an empty menu needs no `G: Default` (the derive would demand
// it); lets `#[serde(default)]` fields omit a never-reached menu.
impl<G> Default for SetPiece<G> {
    fn default() -> Self {
        Self {
            options: Vec::new(),
        }
    }
}

/// One menu button, with visibility gated by the host game's gate vocabulary `G`.
///
/// The explicit serde bound keeps the `#[serde(default)]` fields from making the
/// derive demand a spurious `G: Default`; `G` need only (de)serialize.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "G: Deserialize<'de>", serialize = "G: Serialize"))]
pub struct SetPieceOption<G> {
    /// Button label; a `{cost}` token is filled with the formatted `cost`.
    pub label: String,
    pub hint: String,
    /// Price shown and (when `affordable_only`) gating the button.
    #[serde(default)]
    pub cost: f64,
    /// Conditions that must all hold for the option to *appear* (a failing gate
    /// hides the button entirely).
    #[serde(default)]
    pub gate: Vec<G>,
    /// Conditions that must all hold for the option to be *enabled*. A failing
    /// disable-gate shows the button greyed out (with [`disabled_hint`] if set)
    /// rather than hiding it — so the player still sees the affordance and why
    /// it's unavailable.
    ///
    /// [`disabled_hint`]: Self::disabled_hint
    #[serde(default)]
    pub disable_gate: Vec<G>,
    /// Hint shown in place of [`hint`](Self::hint) while the button is disabled
    /// by a failing [`disable_gate`](Self::disable_gate) (e.g. "Need 40+ rounds").
    #[serde(default)]
    pub disabled_hint: Option<String>,
    /// Disable (rather than hide) the button when cash is short of `cost`.
    #[serde(default)]
    pub affordable_only: bool,
    /// Render with the primary/commit styling.
    #[serde(default)]
    pub primary: bool,
    /// The tag the host maps to an engine op or a screen action.
    pub action: String,
}

/// A visibility condition on a [`SetPieceOption`], evaluated by the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Gate {
    /// The traveller has no horse yet.
    NoHorse,
    /// There is still cargo in the hold.
    HasCargo,
    /// The boat hasn't been broken up.
    HasBoat,
    /// The current stand has this key.
    AtStand(String),
}
