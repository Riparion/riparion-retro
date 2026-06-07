//! Choose a destination. Each voyage takes one month — and the South China
//! Sea is not a friendly place.

use dioxus::prelude::*;

use crate::engine::state::{Mode, PORT_NAMES};
use crate::engine::Game;
use retro_kit::theme::{BTN, BTN_WIDE};

#[component]
pub fn Travel() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut error = use_signal(String::new);
    let current = game.read().state.port;

    rsx! {
        div { class: "flex-1 flex flex-col gap-2 p-3 justify-center max-w-md w-full mx-auto",
            h2 { class: "text-center mb-1", "Taipan, do you wish me to go to:" }
            if !error.read().is_empty() {
                p { class: "text-center chip-danger p-1", "{error}" }
            }
            for port in 1..=7usize {
                if port != current {
                    button {
                        key: "{port}",
                        class: "{BTN_WIDE} py-3",
                        onclick: move |_| {
                            if let Err(e) = game.write().set_sail(port) {
                                error.set(e);
                            }
                        },
                        "{PORT_NAMES[port]}"
                    }
                }
            }
            button {
                class: "{BTN} mt-3",
                onclick: move |_| game.write().mode = Mode::Port,
                "Stay in port"
            }
        }
    }
}
