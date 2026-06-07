//! The port of ROCKET's ASCII distance plot: a vertical track with the
//! lander's glyph at its current altitude, easing down as turns resolve.
//! Position follows the *revealed* telemetry so the strip descends with the
//! paced log instead of snapping to the end-of-turn altitude.

use dioxus::prelude::*;

use crate::engine::state::LogLine;
use crate::engine::Game;
use crate::ui::components::revealed_row;

#[component]
pub fn DescentStrip() -> Element {
    let game = use_context::<Signal<Game>>();
    let pending = use_context::<Signal<Vec<LogLine>>>();
    let row = revealed_row(&game.read(), &pending.read());
    let pct = row.alt_frac * 100.0;
    let burning = row.fuel_raw > 0.0;

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
