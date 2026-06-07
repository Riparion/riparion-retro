//! The port of ROCKET's ASCII distance plot: a vertical track with the
//! lander's glyph at its current altitude, easing down as turns resolve.

use dioxus::prelude::*;

use crate::engine::Game;

#[component]
pub fn DescentStrip() -> Element {
    let game = use_context::<Signal<Game>>();
    let frac = game.read().altitude_frac();
    let pct = frac * 100.0;
    let burning = game.read().fuel_remaining() > 0.0;

    rsx! {
        div { class: "descent-track shrink-0", "aria-hidden": "true",
            div { class: "descent-lander", style: "bottom: {pct}%",
                if burning {
                    "▼"
                } else {
                    "▽"
                }
            }
            div { class: "descent-ground" }
        }
    }
}
