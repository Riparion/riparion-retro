//! The embedded scenario, parsed once and held immutable for the life of the
//! program. Fort Nash's whole content and flow live in `fort_nash.ron`; the
//! engine reads tables, outcomes, and narration off the resulting
//! [`Scenario`](trail_kit::fortnash::Scenario) instead of from hand-written
//! `const`s and `match` arms.
//!
//! The `OnceLock` + `include_str!` + parse boilerplate is shared with kaintuck
//! through [`trail_kit::embed_scenario!`]; only the path, type, and parse fn
//! differ.

trail_kit::embed_scenario!(
    "fort_nash.ron",
    trail_kit::fortnash::Scenario,
    trail_kit::fortnash::parse_scenario
);
