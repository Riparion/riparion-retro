//! Hong Kong godown: shuttle cargo between ship and warehouse.

use dioxus::prelude::*;

use crate::engine::state::{Mode, ITEM_NAMES, WAREHOUSE_CAPACITY};
use crate::engine::Game;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::theme::{ACTION_BAR, BTN, PANEL};

#[component]
pub fn Transfer() -> Element {
    let mut game = use_context::<Signal<Game>>();
    // (item, true = ship → warehouse)
    let mut moving = use_signal(|| Option::<(usize, bool)>::None);

    if let Some((i, to_warehouse)) = moving() {
        let g = game.read();
        let room = WAREHOUSE_CAPACITY - g.state.warehouse_used();
        let max = if to_warehouse {
            g.state.hold[i].min(room)
        } else {
            g.state.warehouse[i]
        };
        let prompt = format!(
            "How much {} shall I move {}, Taipan?",
            ITEM_NAMES[i],
            if to_warehouse { "to the warehouse" } else { "aboard ship" }
        );
        drop(g);
        return rsx! {
            div { class: "flex-1 flex flex-col justify-center",
                NumberEntry {
                    prompt,
                    max,
                    confirm: "Move".to_string(),
                    on_submit: move |qty| {
                        let mut g = game.write();
                        if to_warehouse {
                            g.store_in_warehouse(i, qty);
                        } else {
                            g.load_aboard(i, qty);
                        }
                        moving.set(None);
                    },
                    on_cancel: move |_| moving.set(None),
                }
            }
        };
    }

    let g = game.read();
    let rows: Vec<(usize, i64, i64)> = (0..4).map(|i| (i, g.state.hold[i], g.state.warehouse[i])).collect();
    let used = g.state.warehouse_used();
    let free_hold = g.state.free_hold();
    drop(g);

    rsx! {
        div { class: "flex-1 flex flex-col",
            div { class: "flex-1 overflow-y-auto p-3 flex flex-col gap-2",
                h2 { class: "chip-label text-center", "── HONG KONG GODOWN ──" }
                p { class: "text-xs text-center opacity-70",
                    "Warehouse {used}/{WAREHOUSE_CAPACITY} · Hold space {free_hold}"
                }
                for (i, aboard, stored) in rows {
                    div { key: "{i}", class: "{PANEL} p-2",
                        div { class: "flex justify-between text-sm mb-2",
                            span { "{ITEM_NAMES[i]}" }
                            span { class: "opacity-80", "Ship {aboard} · Godown {stored}" }
                        }
                        div { class: "grid grid-cols-2 gap-2",
                            button {
                                class: "{BTN} text-sm",
                                disabled: aboard == 0,
                                onclick: move |_| moving.set(Some((i, true))),
                                "STORE ▸"
                            }
                            button {
                                class: "{BTN} text-sm",
                                disabled: stored == 0,
                                onclick: move |_| moving.set(Some((i, false))),
                                "◂ LOAD"
                            }
                        }
                    }
                }
            }
            div { class: "{ACTION_BAR}",
                button {
                    class: "{BTN} w-full",
                    onclick: move |_| game.write().mode = Mode::Port,
                    "Done"
                }
            }
        }
    }
}
