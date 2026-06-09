//! "How to Rule" — a quick primer the 1978 original kept terse.

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
                    "Each year you trade, feed your people, tax, and build. Grow your city-state enough and the nobility raises your title. The first to be crowned King or Queen wins — but the years are short, so rule well and rule fast."
                }
                Rule {
                    title: "Trade at market",
                    body: "Grain and land prices swing with the harvest. Buy grain cheap in fat years, sell it dear in lean ones; buy land to grow your tax base.",
                }
                Rule {
                    title: "Feed the city",
                    body: "Release enough grain to meet demand or your serfs starve. A generous surplus (about 1.3× demand) draws migrants — new serfs, merchants, and nobles.",
                }
                Rule {
                    title: "Set your taxes",
                    body: "Customs, sales, and wealth taxes fill the treasury, but heavy taxes choke the growth of merchants and nobles. Harsh justice raises court fees yet drives serfs away and lowers your title.",
                }
                Rule {
                    title: "Build your standing",
                    body: "Marketplaces, mills, palaces, and cathedrals raise public works — the heart of your title score. Mills and markets also pay yearly. Keep soldiers ahead of your land, or Baron Peppone will raid you.",
                }
                p { class: "opacity-70",
                    "Watch the FLORINS chip: deep, lasting debt will see your estate seized. Higher difficulty demands a far greater city before each title is granted."
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
