//! Steady-hand trace: sandbars and the Falls on the river, swamps and the Duck
//! River ford on the Trace. A thin wrapper over `minigames_kit::steady_hands`.

use dioxus::prelude::*;

use minigames_kit::steady_hands::{SteadyHands, SteadyHandsResult};

use crate::engine::tasks::SteadyTask;
use crate::engine::Game;

#[component]
pub fn Steady() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let task = g.pending_steady.unwrap_or(SteadyTask::Sandbar);
    let seed = g.encounter_seed(0x57EA_D70F);
    drop(g);

    let (prompt, tolerance, duration_ms, drift_speed): (&str, f64, u32, f64) = match task {
        SteadyTask::Sandbar => (
            "Sandbar ahead — hold her off the bar with the steering oar.",
            0.13,
            6000,
            1.2,
        ),
        SteadyTask::FallsRun => (
            "The Falls of the Ohio! Run the chute — hold your line through the rapids.",
            0.10,
            7000,
            1.6,
        ),
        SteadyTask::Swamp => (
            "A flooded swamp crossing — keep your footing on the firm ground.",
            0.13,
            6000,
            1.3,
        ),
        SteadyTask::DuckFord => (
            "Fording the Duck River on foot — hold against the current.",
            0.11,
            6500,
            1.5,
        ),
    };

    rsx! {
        SteadyHands {
            prompt: prompt.to_string(),
            tolerance,
            duration_ms,
            drift_speed,
            seed,
            on_complete: move |res: SteadyHandsResult| {
                game.write().resolve_steady(res.steady, res.accuracy);
            },
        }
    }
}
