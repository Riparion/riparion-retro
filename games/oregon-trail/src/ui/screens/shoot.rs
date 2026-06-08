//! The marksmanship reaction game. A word flashes; tap the matching one fast.
//! The tap's reaction time becomes the original's `B1` shooting score.
//!
//! Time is measured with a 20 ms tick counter (gloo-timers) rather than a
//! wall clock, which keeps the engine deterministic and adds no dependencies.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::engine::interaction::{ShotPurpose, SHOT_WORDS};
use crate::engine::Game;
use retro_kit::theme::{BTN_WIDE, SCREEN_CENTERED};

/// Distinct display orders so the target word isn't always in the same spot.
const PERMS: [[usize; 4]; 4] = [[0, 1, 2, 3], [2, 3, 1, 0], [3, 0, 2, 1], [1, 2, 0, 3]];

const TICK_MS: u32 = 20;

fn prompt_for(purpose: Option<ShotPurpose>) -> &'static str {
    match purpose {
        Some(ShotPurpose::Hunt) => "A deer! Take your shot —",
        Some(ShotPurpose::Bandits) => "Bandits! Draw —",
        Some(ShotPurpose::WildAnimals) => "Wolves! Fire —",
        Some(ShotPurpose::Riders { .. }) => "Riders close in! Fire —",
        None => "Fire —",
    }
}

#[component]
pub fn Shoot() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let target = g.shot_word.min(3);
    let purpose = g.shot;
    drop(g);

    let mut ticks = use_signal(|| 0u32);
    let mut answered = use_signal(|| false);

    // Tick the stopwatch until the player fires.
    use_future(move || async move {
        loop {
            if answered() {
                break;
            }
            TimeoutFuture::new(TICK_MS).await;
            ticks += 1;
        }
    });

    let order = PERMS[target];

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-4 items-center",
            p { class: "text-center text-lg opacity-80", "{prompt_for(purpose)}" }
            div { class: "splash-title", "{SHOT_WORDS[target]}" }
            p { class: "text-center text-sm opacity-60", "Tap the word above as fast as you can." }
            div { class: "grid grid-cols-2 gap-2 w-full mt-2",
                for word_idx in order {
                    button {
                        key: "{word_idx}",
                        class: "{BTN_WIDE} py-5 text-lg",
                        onclick: move |_| {
                            if answered() {
                                return;
                            }
                            answered.set(true);
                            let secs = ticks() as f64 * TICK_MS as f64 / 1000.0;
                            game.write().resolve_shot(secs, word_idx == target);
                        },
                        "{SHOT_WORDS[word_idx]}"
                    }
                }
            }
        }
    }
}
