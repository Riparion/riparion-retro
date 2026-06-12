//! Set-piece menus as data: the Falls, Natchez Under-the-Hill, and the trace
//! stands are each a panel of buttons, and a button is just a label, a hint, a
//! cost, a visibility gate, and an action tag the host dispatches. Keeping the
//! menu *structure* here — options, ordering, costs, conditions — lets the host
//! render every set-piece through one generic component, and a scenario reorder
//! or reprice a menu without code.

use serde::{Deserialize, Serialize};

/// The three set-piece menus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Menus {
    pub falls: SetPiece,
    pub natchez: SetPiece,
    pub stand: SetPiece,
}

/// One set-piece: an ordered list of options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetPiece {
    pub options: Vec<SetPieceOption>,
}

/// One menu button.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetPieceOption {
    /// Button label; a `{cost}` token is filled with the formatted `cost`.
    pub label: String,
    pub hint: String,
    /// Price shown and (when `affordable_only`) gating the button.
    #[serde(default)]
    pub cost: f64,
    /// Conditions that must all hold for the option to appear.
    #[serde(default)]
    pub gate: Vec<Gate>,
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
