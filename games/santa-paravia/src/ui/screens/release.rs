//! Distribute grain to the populace for the year. Release enough to meet demand
//! or your people starve; a generous surplus draws migrants to the city.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::components::error_banner::ErrorBanner;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::format::group_thousands;

#[component]
pub fn Release() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let demand = g.state.grain_demand;
    let min = g.release_min();
    let max = g.release_max();
    let err = g.input_error.clone();
    drop(g);

    rsx! {
        div { class: "flex-1 flex flex-col justify-center",
            ErrorBanner { message: err }
            NumberEntry {
                prompt: format!(
                    "How much grain will you release? Your people need {} steres. Release between {} and {} (keep 20% in reserve).",
                    group_thousands(demand),
                    group_thousands(min),
                    group_thousands(max),
                ),
                max,
                preview: Callback::new(move |v: i64| {
                    if v < min {
                        format!("below the {} minimum — release more", group_thousands(min))
                    } else if v < demand {
                        "short of demand — your people will starve".to_string()
                    } else if v > (demand as f64 * 1.3) as i64 {
                        "a generous surplus — migrants will come".to_string()
                    } else {
                        "enough to keep the city fed".to_string()
                    }
                }),
                confirm: "DISTRIBUTE".to_string(),
                on_submit: move |v: i64| {
                    let _ = game.write().submit_release(v);
                },
            }
        }
    }
}
