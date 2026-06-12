//! Trading at a frontier station. Every dollar buys two-thirds its value in goods.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::Game;
use retro_kit::components::spend_row::SpendRow;
use retro_kit::theme::{BTN, BTN_PRIMARY, SCREEN};

const ITEMS: [&str; 4] = ["Provisions", "Powder & shot", "Winter clothing", "Supplies"];

#[component]
pub fn Fort() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut spend = use_signal(|| vec![0i64; 4]);
    let cash = game.read().state.cash.max(0.0) as i64;
    // Frontier prices and ammo conversion, from the scenario (display only).
    let value = scenario().fort.value_num / scenario().fort.value_den;
    let bpd = scenario().start.bullets_per_dollar;

    let vals = spend.read().clone();
    let total: i64 = vals.iter().sum();
    let remaining = cash - total;
    let ok = remaining >= 0;

    let buy = move |_| {
        let v = spend.read().clone();
        game.write()
            .buy_at_fort(v[0] as f64, v[1] as f64, v[2] as f64, v[3] as f64);
    };

    rsx! {
        div { class: "{SCREEN} gap-3",
            div { class: "text-center",
                h2 { class: "text-lg tracking-widest", "FRONTIER STATION" }
                p { class: "opacity-80 text-sm", "Goods cost more out here — you get ⅔ value." }
            }
            div { class: "flex flex-col gap-2",
                for (i, label) in ITEMS.iter().enumerate() {
                    SpendRow {
                        key: "{i}",
                        label: label.to_string(),
                        note: Some(if i == 1 {
                            format!("get {} rounds", (vals[1] as f64 * value * bpd) as i64)
                        } else {
                            format!("get {} units", (vals[i] as f64 * value) as i64)
                        }),
                        value: vals[i],
                        on_input: move |n| {
                            spend.write()[i] = n;
                        },
                    }
                }
            }
            div { class: "text-center text-lg",
                span { class: if remaining < 0 { "chip-danger" } else { "" }, "Coin left: ${remaining}" }
            }
            div { class: "flex gap-2",
                button {
                    class: "{BTN} flex-1 py-3",
                    onclick: move |_| game.write().leave_fort(),
                    "Leave"
                }
                button {
                    class: "{BTN_PRIMARY} flex-1 py-3",
                    disabled: !ok || total == 0,
                    onclick: buy,
                    "Buy & move on ▸"
                }
            }
        }
    }
}
