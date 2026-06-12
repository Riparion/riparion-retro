//! The weekly hub: hunt, stop at a station, or press on down the road. The
//! buttons come from the scenario's `trail` menu, rendered through the generic
//! `SetPieceMenu`; the station button shows only when a station is offered this
//! week and the hunt button only with powder enough (40+ rounds) to hunt.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::Game;
use crate::ui::components::set_piece_menu::SetPieceMenu;
use retro_kit::theme::SCREEN_CENTERED;

#[component]
pub fn Trail() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let terrain = g.state.terrain();
    let progress =
        (g.state.miles.max(0.0) / scenario().trail.total_miles * 100.0).min(100.0) as i64;
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
            SetPieceMenu {
                options: scenario().menus.trail.options.as_slice(),
                onselect: move |action: String| {
                    let mut w = game.write();
                    match action.as_str() {
                        "fort" => w.choose_fort(),
                        "hunt" => {
                            // The button only shows with powder enough, so this
                            // never errors; the guard in `choose_hunt` stays anyway.
                            let _ = w.choose_hunt();
                        }
                        "press-on" => w.choose_continue(),
                        _ => {}
                    }
                },
            }
        }
    }
}
