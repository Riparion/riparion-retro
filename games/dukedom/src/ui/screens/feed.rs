//! Allocate grain to feed the peasants through winter.

use dioxus::prelude::*;

use crate::engine::Game;
use crate::ui::components::error_banner::ErrorBanner;
use retro_kit::components::number_entry::NumberEntry;

#[component]
pub fn Feed() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let peasants = g.state.n_p;
    let grain = g.state.n_g;
    let err = g.input_error.clone();
    drop(g);

    rsx! {
        div { class: "flex-1 flex flex-col justify-center",
            ErrorBanner { message: err }
            NumberEntry {
                prompt: format!(
                    "Winter is coming. How much grain will you give your {peasants} peasants? (Granary: {grain} HL.)"
                ),
                max: grain,
                preview: Callback::new(move |v: i64| {
                    let per = v / peasants.max(1);
                    let note = if per >= 13 {
                        "they'll survive"
                    } else if per >= 11 {
                        "lean, but calm"
                    } else {
                        "too little — they'll starve"
                    };
                    format!("{per} HL per peasant — {note}")
                }),
                confirm: "FEED".to_string(),
                on_submit: move |v: i64| {
                    let _ = game.write().submit_feed(v);
                },
            }
        }
    }
}
