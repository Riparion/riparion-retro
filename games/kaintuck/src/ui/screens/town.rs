//! A river-town hub: the local market, and what you can do at the landing.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::state::{fmt_money, Mode, GOOD_NAMES};
use crate::engine::Game;
use retro_kit::components::seg_button::SegButton;
use retro_kit::theme::{ACTION_BAR, BTN, PANEL};

#[component]
pub fn Town() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let town = s.town;
    let has_moneylender = scenario().river.towns[town].moneylender;
    let convoy = s.river_convoy;
    // Quote the prices the player actually transacts at: the ask to buy, the bid
    // to sell (spread folded in), not the bare mid in `state.prices`.
    let rows: Vec<(usize, String, String, i64)> = (0..GOOD_NAMES.len())
        .map(|i| (i, fmt_money(g.buy_price(i)), fmt_money(g.sell_price(i)), s.hold[i]))
        .collect();
    drop(g);

    rsx! {
        div { class: "flex-1 flex flex-col",
            div { class: "flex-1 overflow-y-auto p-3 flex flex-col gap-3",
                div { class: "{PANEL} p-3",
                    h2 { class: "chip-label mb-2", "── THE MARKET ──" }
                    table { class: "w-full text-sm",
                        thead {
                            tr { class: "chip-label text-left",
                                th { "CARGO" }
                                th { class: "text-right", "BUY" }
                                th { class: "text-right", "SELL" }
                                th { class: "text-right", "ABOARD" }
                            }
                        }
                        tbody {
                            for (i, buy, sell, aboard) in rows {
                                tr { key: "{i}",
                                    td { "{GOOD_NAMES[i]}" }
                                    td { class: "text-right", "{buy}" }
                                    td { class: "text-right", "{sell}" }
                                    td { class: "text-right", "{aboard}" }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "{ACTION_BAR}",
                div { class: "chip-label mb-1", "── COMPANY ──" }
                div { class: "flex gap-2",
                    SegButton {
                        label: "Sail alone".to_string(),
                        active: !convoy,
                        onclick: move |_| game.write().set_river_convoy(false),
                    }
                    SegButton {
                        label: "In company".to_string(),
                        active: convoy,
                        onclick: move |_| game.write().set_river_convoy(true),
                    }
                }
                p { class: "text-xs opacity-60 mb-2",
                    "A convoy draws off river pirates and halves their take — but forming up costs a day on the water."
                }
                div { class: "grid grid-cols-2 gap-2",
                    button {
                        class: "{BTN} col-span-2",
                        onclick: move |_| game.write().mode = Mode::Trade { can_buy: true, can_sell: true },
                        "TRADE ⇄"
                    }
                    if has_moneylender {
                        button {
                            class: "{BTN} col-span-2",
                            onclick: move |_| game.write().mode = Mode::Moneylender,
                            "MONEYLENDER"
                        }
                    }
                    button {
                        class: "crt-btn crt-btn-primary col-span-2",
                        onclick: move |_| game.write().depart(),
                        "CAST OFF ⛵ ▸"
                    }
                }
            }
        }
    }
}
