//! Cover art for the current screen. [`cover_keys`] builds an ordered list of
//! candidate keys — most-specific narrative key first, then the general `Mode`
//! key — and [`Cover`] renders the first one that has a bundled jpg. Drop a
//! `<key>.jpg` into `assets/covers/` to light a screen up; nothing else to wire.
//! A debug-only key strip prints the candidates so you can name files by walking
//! the game.

use dioxus::prelude::*;

use crate::engine::interaction::Interaction;
use crate::engine::state::{Mode, TOWN_SLUGS, NUM_RIVER_TOWNS};
use crate::engine::tasks::{
    HeaveTask, HotColdTask, HunterTask, QuickTask, SteadyTask, TimingTask,
};
use crate::engine::Game;

// build.rs writes `cover_asset(key) -> Option<Asset>`, one arm per jpg present.
include!(concat!(env!("OUT_DIR"), "/covers.rs"));

fn mode_key(mode: &Mode) -> &'static str {
    match mode {
        Mode::Splash => "splash",
        Mode::NewGame => "new-game",
        Mode::Pittsburgh => "pittsburgh",
        Mode::Town => "town",
        Mode::Trade { .. } => "trade",
        Mode::Moneylender => "moneylender",
        Mode::Falls => "falls",
        Mode::GrandTower => "grand-tower",
        Mode::CaveInRock => "cave-in-rock",
        Mode::Natchez => "natchez",
        Mode::TraceHub => "trace",
        Mode::Stand => "stand",
        Mode::Steady => "steady",
        Mode::Quick => "quick",
        Mode::Crowd => "crowd",
        Mode::Timing => "timing",
        Mode::Sequence => "sequence",
        Mode::Brigade => "brigade",
        Mode::Heave => "heave",
        Mode::HotCold => "hot-cold",
        Mode::Hunter => "hunter",
        Mode::Interaction => "interaction",
        Mode::GameOver => "game-over",
    }
}

fn town_slug(town: usize) -> &'static str {
    TOWN_SLUGS[town.min(NUM_RIVER_TOWNS - 1)]
}

/// Candidate cover keys for the current screen+state, most specific first.
pub fn cover_keys(game: &Game) -> Vec<String> {
    let mut keys = Vec::new();
    match game.mode {
        Mode::Town => keys.push(format!("town-{}", town_slug(game.state.town))),
        Mode::Stand => {
            if let Some(stand) = game.current_stand() {
                keys.push(format!("trace-{}", stand.key()));
            }
        }
        Mode::Steady => {
            if let Some(task) = game.steady_task() {
                let v = match task {
                    SteadyTask::FallsRun => "falls-run",
                    SteadyTask::CaveRun => "cave-run",
                    SteadyTask::DuckFord => "duck-ford",
                };
                keys.push(format!("steady-{v}"));
            }
        }
        Mode::Quick => {
            if let Some(task) = game.quick_task() {
                let v = match task {
                    QuickTask::Pirates => "pirates",
                };
                keys.push(format!("quick-{v}"));
            }
        }
        Mode::Heave => {
            if let Some(task) = game.heave_task() {
                let v = match task {
                    HeaveTask::Ground => "sandbar",
                    HeaveTask::Cordelle => "cordelle",
                };
                keys.push(format!("heave-{v}"));
            }
        }
        Mode::HotCold => {
            if let Some(task) = game.hotcold_task() {
                let v = match task {
                    HotColdTask::Swamp => "swamp",
                    HotColdTask::MasonSearch => "mason",
                };
                keys.push(format!("hot-cold-{v}"));
            }
        }
        Mode::Hunter => {
            if let Some(task) = game.hunter_task() {
                let v = match task {
                    HunterTask::Harpe => "harpe",
                    HunterTask::Pot => "trace-hunt",
                };
                keys.push(format!("hunter-{v}"));
            }
        }
        Mode::Crowd => keys.push("crowd-side-trail".to_string()),
        Mode::Timing => {
            if let Some(task) = game.timing_task() {
                let v = match task {
                    TimingTask::Gamble => "gamble",
                    TimingTask::Dose => "dose",
                    TimingTask::CaveTell => "cave-tell",
                };
                keys.push(format!("timing-{v}"));
            }
        }
        Mode::Sequence => keys.push("sequence-patch".to_string()),
        Mode::Brigade => keys.push("brigade-bail".to_string()),
        Mode::Interaction => {
            if let Some(Interaction::Message {
                cover: Some(slug), ..
            }) = game.pending.front()
            {
                keys.push(format!("interaction-{slug}"));
            }
        }
        Mode::GameOver => {
            if let Some(end) = &game.outcome {
                keys.push(format!("game-over-{}", end.cause_kind.key()));
            }
        }
        _ => {}
    }
    keys.push(mode_key(&game.mode).to_string());
    keys
}

#[component]
pub fn Cover() -> Element {
    let game = use_context::<Signal<Game>>();
    let keys = cover_keys(&game.read());
    let resolved = keys
        .iter()
        .find_map(|k| cover_asset(k).map(|asset| (k.clone(), asset)));

    rsx! {
        if let Some((_, asset)) = resolved.clone() {
            img {
                class: "block w-full max-w-[21rem] mx-auto aspect-square object-cover",
                src: asset,
            }
        }
        if cfg!(debug_assertions) {
            div {
                class: "fixed bottom-0 inset-x-0 z-50 pointer-events-none px-2 py-1 \
                        text-[10px] leading-tight font-mono text-center \
                        bg-black/60 text-amber-200/90",
                for key in keys.iter() {
                    span {
                        class: if resolved.as_ref().is_some_and(|(rk, _)| rk == key) {
                            "text-emerald-300 font-bold"
                        } else {
                            "opacity-70"
                        },
                        if resolved.as_ref().is_some_and(|(rk, _)| rk == key) {
                            "✓ {key}  "
                        } else {
                            "{key}  "
                        }
                    }
                }
            }
        }
    }
}
