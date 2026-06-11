//! Timing-bar game: the Under-the-Hill gamble at Natchez and measuring out a
//! dose of medicine on the Trace. A thin wrapper over `minigames_kit::timing_bar`.

use dioxus::prelude::*;

use minigames_kit::timing_bar::{TimingBar, TimingResult};

use crate::engine::tasks::TimingTask;
use crate::engine::Game;

#[component]
pub fn Timing() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let task = g.timing_task().unwrap_or(TimingTask::Dose);
    let seed = g.encounter_seed(0x7131_46A1);
    drop(g);

    let (prompt, action, tolerance, period_ms): (&str, &str, f64, u32) = match task {
        TimingTask::Gamble => (
            "Under-the-Hill — lay your stake and call it as the wheel comes round.",
            "CALL IT",
            0.10,
            1300,
        ),
        TimingTask::Dose => (
            "Swamp fever. Measure out the dose — too much or too little does no good.",
            "POUR",
            0.12,
            1500,
        ),
    };

    rsx! {
        TimingBar {
            prompt: prompt.to_string(),
            action: action.to_string(),
            tolerance,
            period_ms,
            seed,
            on_strike: move |res: TimingResult| {
                game.write().resolve_timing(res.hit, res.accuracy);
            },
        }
    }
}
