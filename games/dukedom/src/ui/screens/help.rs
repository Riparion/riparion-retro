//! "How to Rule" — the rules the 1980 original never bothered to print.

use dioxus::prelude::*;

use crate::ui::Overlay;
use retro_kit::theme::{BTN_PRIMARY, PANEL, SCREEN};

#[component]
pub fn Help() -> Element {
    let mut overlay = use_context::<Signal<Overlay>>();

    rsx! {
        div { class: "{SCREEN} gap-3",
            h2 { class: "text-center text-lg tracking-widest", "HOW TO RULE" }
            div { class: "{PANEL} p-3 flex flex-col gap-3 text-sm leading-snug",
                p {
                    "Each year you make three decisions, then watch the realm react. Survive 45 years with the King appeased to retire in glory — or defeat his army to seize the crown yourself."
                }
                Rule {
                    title: "Feed the peasants",
                    body: "Give each peasant about 13 HL of grain to see them through winter. Below 13, some starve; below 11, the rest grow restless. Starvation and unrest depose Dukes.",
                }
                Rule {
                    title: "Buy or sell land",
                    body: "Land is priced in grain. Buy in good years to grow; sell in lean ones to eat. You make one land deal per year.",
                }
                Rule {
                    title: "Plant the fields",
                    body: "Each HA needs 2 HL of seed, and each peasant can work at most 4 HA. Rested land grows more fertile; cropped land tires. Plant within your means.",
                }
                Rule {
                    title: "Then fate intervenes",
                    body: "Rats, seven-year locusts, plague, and rival Dukes all take their turn. The High King levies grain and peasants — and if you provoke him, his army marches.",
                }
                p { class: "opacity-70",
                    "Watch the MOOD chip: a seething populace will overthrow you. Keep grain in reserve — empty granaries are fatal."
                }
            }
            button {
                class: "{BTN_PRIMARY} py-3",
                onclick: move |_| overlay.set(Overlay::Splash),
                "◂ BACK"
            }
        }
    }
}

#[component]
fn Rule(title: &'static str, body: &'static str) -> Element {
    rsx! {
        div {
            p { class: "font-bold", "{title}" }
            p { class: "opacity-80", "{body}" }
        }
    }
}
