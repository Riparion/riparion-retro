//! Cover art for the current screen. Given the game state, [`cover_keys`] builds
//! an ordered list of candidate keys — most-specific narrative key first, then
//! the general `Mode` key — and [`Cover`] renders the first one that has a
//! bundled jpg. Drop a `<key>.jpg` into `assets/covers/` to light a screen up;
//! nothing else to wire. See OREGONTRAIL_IMAGE_KEYS.md for the full key list.
//!
//! In debug builds a small strip at the bottom of the screen prints the
//! candidate keys (marking which one resolved to art), so you can walk the game
//! and read off exactly what to name each file. It is compiled out of release.

use dioxus::prelude::*;

use crate::engine::interaction::{Interaction, ShotPurpose};
use crate::engine::state::Mode;
use crate::engine::{BrigadeTask, Game, Illness, SteadyTask};

// build.rs writes `cover_asset(key) -> Option<Asset>`, one arm per jpg present.
include!(concat!(env!("OUT_DIR"), "/covers.rs"));

/// The general cover key for a screen — the `Mode` variant in kebab-case.
fn mode_key(mode: &Mode) -> &'static str {
    match mode {
        Mode::Splash => "splash",
        Mode::NewGame => "new-game",
        Mode::Outfit => "outfit",
        Mode::Trail => "trail",
        Mode::Fort => "fort",
        Mode::Eat => "eat",
        Mode::Shoot => "shoot",
        Mode::Hunt => "hunt",
        Mode::Flee => "flee",
        Mode::Climb => "climb",
        Mode::Fog => "fog",
        Mode::Splint => "splint",
        Mode::Dose => "dose",
        Mode::Steady => "steady",
        Mode::Brigade => "brigade",
        Mode::Riders => "riders",
        Mode::Interaction => "interaction",
        Mode::GameOver => "game-over",
    }
}

/// Candidate cover keys for the current screen+state, most specific first. The
/// renderer picks the first one that has art; the general `Mode` key is always
/// the final fallback.
pub fn cover_keys(game: &Game) -> Vec<String> {
    let mut keys = Vec::new();
    let general = mode_key(&game.mode);

    match game.mode {
        Mode::Trail => {
            // Location-specific art for the hub, by the stretch of country.
            keys.push(format!("trail-{}", game.state.terrain_kind().key()));
        }
        Mode::Steady => {
            if let Some(task) = game.pending_steady {
                let v = match task {
                    SteadyTask::Snakebite => "snakebite",
                    SteadyTask::Ford => "ford",
                    SteadyTask::OxLeg => "ox-leg",
                };
                keys.push(format!("steady-{v}"));
            }
        }
        Mode::Brigade => {
            if let Some(task) = game.pending_brigade {
                let v = match task {
                    BrigadeTask::Fire => "fire",
                    BrigadeTask::Rains => "rains",
                    BrigadeTask::Blizzard => "blizzard",
                };
                keys.push(format!("brigade-{v}"));
            }
        }
        Mode::Shoot => {
            if let Some(shot) = game.shot {
                let v = match shot {
                    ShotPurpose::Bandits => "bandits",
                    ShotPurpose::WildAnimals => "wild-animals",
                    ShotPurpose::Riders { circle: true } => "riders-circled",
                    ShotPurpose::Riders { circle: false } => "riders-open",
                };
                keys.push(format!("shoot-{v}"));
            }
        }
        Mode::Riders => {
            if let Some(enc) = game.riders {
                let v = if enc.looks_hostile { "hostile" } else { "friendly" };
                keys.push(format!("riders-{v}"));
            }
        }
        Mode::Dose => {
            if let Some(illness) = game.pending_illness {
                let v = match illness {
                    Illness::Mild => "mild",
                    Illness::Bad => "bad",
                    Illness::Serious => "serious",
                };
                keys.push(format!("dose-{v}"));
            }
        }
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

    keys.push(general.to_string());
    keys
}

/// Renders the most-specific available cover for the current screen (nothing if
/// none exists), plus a debug-only key strip.
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
