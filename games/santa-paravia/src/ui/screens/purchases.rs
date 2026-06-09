//! Invest the treasury in public works and defence. Buildings raise your public
//! standing (and your title); soldiers keep the Baron at bay.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN_PRIMARY, PANEL, SCREEN};

#[component]
pub fn Purchases() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let treasury = s.treasury;
    let serfs = s.serfs;
    let counts = (s.marketplaces, s.mills, s.palace, s.cathedral, s.soldiers);
    drop(g);
    let (mk, ml, pl, ca, so) = counts;

    rsx! {
        div { class: "{SCREEN} gap-2",
            h2 { class: "text-center text-lg tracking-widest", "STATE PURCHASES" }
            div { class: "{PANEL} p-2 text-center",
                "Treasury: {group_thousands(treasury)} florins"
            }

            MenuButton {
                title: format!("Marketplace ({mk}) — 1,000 fl"),
                hint: "+5 merchants, +75 fl / year, +standing",
                disabled: treasury < 1000,
                onclick: move |_| game.write().buy_marketplace(),
            }
            MenuButton {
                title: format!("Woolen mill ({ml}) — 2,000 fl"),
                hint: "+55–305 fl / year, +standing",
                disabled: treasury < 2000,
                onclick: move |_| game.write().buy_mill(),
            }
            MenuButton {
                title: format!("Palace ({pl}) — 3,000 fl"),
                hint: "+0–2 nobles, +standing",
                disabled: treasury < 3000,
                onclick: move |_| game.write().buy_palace(),
            }
            MenuButton {
                title: format!("Cathedral ({ca}) — 5,000 fl"),
                hint: "+0–6 clergy, +standing",
                disabled: treasury < 5000,
                onclick: move |_| game.write().buy_cathedral(),
            }
            MenuButton {
                title: format!("Equip a platoon — 500 fl"),
                hint: format!("+20 soldiers (now {so}) from 20 serfs"),
                disabled: treasury < 500 || serfs < 20,
                onclick: move |_| game.write().buy_soldiers(),
            }

            button {
                class: "{BTN_PRIMARY} py-4 text-lg mt-1",
                onclick: move |_| game.write().done_purchases(),
                "CLOSE THE YEAR ▸"
            }
        }
    }
}
