//! Title screen with the local fleet of past Taipans.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use retro_kit::theme::BTN_PRIMARY;

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let scores = use_hook(storage::high_scores);

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-6 p-6 text-center",
            div {
                h1 { class: "splash-title", "TAIPAN!" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "A game of trade and treachery on the South China Sea, 1860."
                }
            }
            button {
                class: "{BTN_PRIMARY} max-w-xs text-xl",
                onclick: move |_| game.write().mode = Mode::NewGame,
                "SET SAIL"
            }
            if !scores.is_empty() {
                div { class: "crt-panel p-3 w-full max-w-xs text-left",
                    h2 { class: "chip-label mb-2 text-center", "── HALL OF TAIPANS ──" }
                    for (i, hs) in scores.iter().enumerate() {
                        div { key: "{i}", class: "flex justify-between text-sm gap-2",
                            span { class: "truncate", "{hs.firm}" }
                            span { class: "opacity-80 shrink-0", "{hs.score} · {hs.rank}" }
                        }
                    }
                }
            }
            p { class: "text-xs opacity-60", "After the 1982 Apple ][ classic by Art Canfil" }
        }
    }
}
