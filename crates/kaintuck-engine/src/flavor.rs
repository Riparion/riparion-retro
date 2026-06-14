//! Shared helpers for composing data-driven flavor lines (trader [`gossip`](crate::gossip)
//! and dockside [`rumor`](crate::rumor)s) from the scenario's phrasing pools, so
//! good/town name resolution and template selection live in one place instead of
//! being re-derived per module. All RNG-free and deterministic.

use crate::scenario_data::scenario;
use crate::state::GOOD_NAMES;

/// Lowercased display name of a good, or `""` if the index is out of range.
pub fn good_name(good: usize) -> String {
    GOOD_NAMES.get(good).map(|g| g.to_lowercase()).unwrap_or_default()
}

/// Display name of a river town, or `""` if the index is out of range.
pub fn town_name(town: usize) -> String {
    scenario()
        .river
        .towns
        .get(town)
        .map(|t| t.name.clone())
        .unwrap_or_default()
}

/// Index of a good by its exact display name, if known.
pub fn good_index(name: &str) -> Option<usize> {
    GOOD_NAMES.iter().position(|n| *n == name)
}

/// Pick one entry from a slice by an RNG-free rotation seed — the `% len`
/// reduction lives here so the choice is identical on wasm and native. Callers
/// must guard against an empty slice (`!templates.is_empty()`).
pub fn pick(items: &[String], seed: u64) -> &str {
    &items[(seed % items.len() as u64) as usize]
}
