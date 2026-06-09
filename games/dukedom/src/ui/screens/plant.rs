//! Choose how many acres to sow this spring.

use dioxus::prelude::*;

use crate::engine::Game;
use crate::ui::components::error_banner::ErrorBanner;
use retro_kit::components::number_entry::NumberEntry;

#[component]
pub fn Plant() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let land = g.state.n_l;
    let peasants = g.state.n_p;
    let grain = g.state.n_g;
    let err = g.input_error.clone();
    drop(g);

    // Limited by land, by labour (4 HA/peasant), and by seed grain (2 HL/HA).
    let max = land.min(4 * peasants).min(grain / 2).max(0);

    rsx! {
        div { class: "flex-1 flex flex-col justify-center",
            ErrorBanner { message: err }
            NumberEntry {
                prompt: format!(
                    "How many acres will you plant? Your {peasants} peasants can work up to {} HA.",
                    4 * peasants
                ),
                max,
                preview: Callback::new(move |v: i64| format!("uses {} HL of seed grain", 2 * v)),
                confirm: "PLANT".to_string(),
                on_submit: move |v: i64| {
                    let _ = game.write().submit_plant(v);
                },
            }
        }
    }
}
