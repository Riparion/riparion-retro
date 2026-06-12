//! The embedded Kaintuck scenario. The RON content/flow file is compiled into
//! the binary and parsed once, lazily, into an immutable [`Scenario`] behind a
//! [`OnceLock`] — the same shape as the adventure game's `data()` (see
//! `games/adventure/src/engine/data.rs`). The engine reads it ad hoc through
//! [`scenario()`] rather than holding a borrow, so [`Game`](super::Game) stays
//! `Serialize`.

use std::sync::OnceLock;

use trail_kit::Scenario;

/// The verbatim scenario source, embedded in the binary.
const KAINTUCK_RON: &str = include_str!("kaintuck.ron");

/// The parsed, immutable scenario. Parsed once on first access; a malformed RON
/// file is a programming error, so a parse failure panics with the span.
pub fn scenario() -> &'static Scenario {
    static SCENARIO: OnceLock<Scenario> = OnceLock::new();
    SCENARIO.get_or_init(|| {
        trail_kit::parse_scenario(KAINTUCK_RON).expect("kaintuck.ron failed to parse")
    })
}
