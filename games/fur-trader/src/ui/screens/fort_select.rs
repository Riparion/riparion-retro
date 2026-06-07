//! "YOU MAY TRADE YOUR FURS AT FORT 1, FORT 2, OR FORT 3" — three tappable
//! route cards (lines 1100–1110).

use dioxus::prelude::*;

use crate::engine::state::{fort_info, FORT_ORDER};
use crate::engine::{fmt_dollars, Game};
use retro_kit::theme::PANEL;

#[component]
pub fn FortSelect() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let savings = game.read().state.savings;

    rsx! {
        div { class: "flex-1 flex flex-col gap-2 p-3 overflow-y-auto",
            p { class: "text-sm opacity-80",
                "You may trade your furs at one of three forts. Where do you take them?"
            }
            for (i, fort) in FORT_ORDER.iter().copied().enumerate() {
                {
                    let info = fort_info(fort);
                    let cost = info.cost();
                    rsx! {
                        button {
                            class: "{PANEL} fort-card w-full text-left p-3 flex flex-col gap-1",
                            onclick: move |_| game.write().choose_fort(fort),
                            div { class: "flex justify-between items-baseline",
                                span { class: "font-bold", "FORT {i + 1} — {info.name}" }
                                span { class: "text-sm opacity-80", "{info.city}" }
                            }
                            div { class: "flex justify-between text-sm",
                                span {
                                    class: if cost > savings { "chip-danger" },
                                    "Outfitting {fmt_dollars(cost)}"
                                }
                                span { class: "opacity-80", "{info.risk}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
