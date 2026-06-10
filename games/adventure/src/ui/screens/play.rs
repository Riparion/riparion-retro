//! The one gameplay screen: the transcript fills the space, the command line is
//! pinned to the bottom, and when the engine is waiting on a yes/no question we
//! surface quick YES / NO buttons (you can still type them).

use dioxus::prelude::*;

use crate::engine::Game;
use crate::ui::components::command_input::CommandInput;
use crate::ui::components::transcript::Transcript;

#[component]
pub fn Play() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let pending = game.read().pending.is_some();

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0",
            Transcript {}
            if pending {
                div { class: "flex gap-2 px-2 pt-2",
                    button {
                        class: "crt-btn crt-btn-primary flex-1",
                        onclick: move |_| {
                            game.write().command("yes");
                        },
                        "YES"
                    }
                    button {
                        class: "crt-btn flex-1",
                        onclick: move |_| {
                            game.write().command("no");
                        },
                        "NO"
                    }
                }
            }
            CommandInput {
                on_submit: move |cmd: String| {
                    game.write().command(&cmd);
                },
            }
        }
    }
}
