//! Fort Nash's set-piece menus as data: the weekly Trail hub and the war-party
//! tactics panel are each a list of buttons. The button/menu *shape* is the
//! shared, gate-generic [`SetPiece`]/[`SetPieceOption`] from the crate root
//! (re-exported here); Fort Nash supplies only its own [`Gate`] vocabulary and
//! the two menus that hang off it.

use serde::{Deserialize, Serialize};

pub use crate::setpiece::{SetPiece, SetPieceOption};

/// The data-driven set-piece menus. Fort and Eat keep their numeric spend inputs
/// and stay host-rendered; only the pure-button menus live here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Menus {
    /// The weekly hub: hunt, stop at a station, or press on.
    pub trail: SetPiece<Gate>,
    /// The war-party tactics panel.
    pub riders: SetPiece<Gate>,
}

/// A visibility condition on a [`SetPieceOption`], evaluated by the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Gate {
    /// A station to resupply at is offered this week.
    FortAvailable,
    /// There's powder enough on hand to go hunting (40+ rounds).
    CanHunt,
}
