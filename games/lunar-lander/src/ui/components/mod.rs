pub mod descent_strip;
pub mod status_bar;

use crate::engine::state::{LogLine, TurnRow};
use crate::engine::Game;

/// The most recent telemetry the player has actually been shown: the last
/// row among the pending (mid-playback) lines, else the committed log, else
/// a report built from the live state. Keeps the status chips and descent
/// strip in step with the paced log instead of jumping to end-of-turn values.
pub fn revealed_row(game: &Game, pending: &[LogLine]) -> TurnRow {
    pending
        .iter()
        .rev()
        .chain(game.log.iter().rev())
        .find_map(|line| match line {
            LogLine::Row(row) => Some(row.clone()),
            LogLine::Banner(_) => None,
        })
        .unwrap_or_else(|| game.current_report())
}
