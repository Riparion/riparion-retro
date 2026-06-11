//! The weekly hub: hunt, stop at a station, or press on down the road.

use dioxus::prelude::*;

use crate::engine::state::TRAIL_MILES;
use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::theme::SCREEN_CENTERED;

#[component]
pub fn Trail() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut error = use_signal(String::new);
    let g = game.read();
    let fort = g.state.fort_available();
    let can_hunt = g.state.bullets > 39.0;
    let terrain = g.state.terrain();
    let progress = (g.state.miles.max(0.0) / TRAIL_MILES * 100.0).min(100.0) as i64;
    drop(g);

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            div { class: "crt-panel p-3 text-center",
                div { class: "chip-label", "─ WHERE YOU ARE ─" }
                div { class: "text-lg mt-1", "{terrain}" }
                div { class: "trail-track mt-2",
                    div { class: "trail-fill", style: "width: {progress}%" }
                    div { class: "trail-wagon", style: "left: {progress}%", "🛞" }
                }
                div { class: "text-xs opacity-60 mt-1", "{progress}% of the way to the French Lick" }
            }
            h2 { class: "text-center text-lg", "The road runs on. What now?" }
            if !error.read().is_empty() {
                p { class: "text-center chip-danger p-1", "{error}" }
            }
            if fort {
                MenuButton {
                    title: "🏰 Stop at the next station".to_string(),
                    hint: "Trade for supplies — but goods cost more here".to_string(),
                    onclick: move |_| game.write().choose_fort(),
                }
            }
            MenuButton {
                title: "🦌 Go hunting".to_string(),
                hint: (if can_hunt { "Spend powder and time for meat" } else { "Need 40+ rounds" })
                    .to_string(),
                disabled: !can_hunt,
                onclick: move |_| {
                    if let Err(e) = game.write().choose_hunt() {
                        error.set(e);
                    }
                },
            }
            MenuButton {
                title: "🛞 Press on down the road ▸".to_string(),
                primary: true,
                onclick: move |_| game.write().choose_continue(),
            }
        }
    }
}
