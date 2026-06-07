//! The Comprador's report: prices, holdings, and everything you can do in port.

use dioxus::prelude::*;

use crate::engine::state::{fmt_money, Mode, HONG_KONG, ITEM_NAMES};
use crate::engine::Game;
use retro_kit::theme::{ACTION_BAR, BTN, PANEL};

#[component]
pub fn Port() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let at_hk = s.port == HONG_KONG;
    let can_retire = g.can_retire();
    let warehouse_used = s.warehouse_used();
    let rows: Vec<(usize, String, i64, i64)> = (0..4)
        .map(|i| (i, fmt_money(s.prices[i]), s.hold[i], s.warehouse[i]))
        .collect();
    drop(g);

    rsx! {
        div { class: "flex-1 flex flex-col",
            div { class: "flex-1 overflow-y-auto p-3 flex flex-col gap-3",
                div { class: "{PANEL} p-3",
                    h2 { class: "chip-label mb-2", "── COMPRADOR'S REPORT ──" }
                    table { class: "w-full text-sm",
                        thead {
                            tr { class: "chip-label text-left",
                                th { "GOODS" }
                                th { class: "text-right", "PRICE" }
                                th { class: "text-right", "ABOARD" }
                                if at_hk {
                                    th { class: "text-right", "GODOWN" }
                                }
                            }
                        }
                        tbody {
                            for (i, price, aboard, stored) in rows {
                                tr { key: "{i}", class: "price-row",
                                    td { "{ITEM_NAMES[i]}" }
                                    td { class: "text-right", "{price}" }
                                    td { class: "text-right", "{aboard}" }
                                    if at_hk {
                                        td { class: "text-right", "{stored}" }
                                    }
                                }
                            }
                        }
                    }
                    if at_hk {
                        p { class: "text-xs opacity-70 mt-2", "Warehouse: {warehouse_used} of 10,000 used" }
                    }
                }
                if can_retire {
                    button {
                        class: "{BTN} w-full crt-btn-primary",
                        onclick: move |_| game.write().retire(),
                        "★ RETIRE — YOU'RE A MILLIONAIRE! ★"
                    }
                }
            }
            div { class: "{ACTION_BAR}",
                div { class: "grid grid-cols-2 gap-2",
                    button {
                        class: "{BTN}",
                        onclick: move |_| game.write().mode = Mode::Trade { buying: true },
                        "BUY"
                    }
                    button {
                        class: "{BTN}",
                        onclick: move |_| game.write().mode = Mode::Trade { buying: false },
                        "SELL"
                    }
                    if at_hk {
                        button {
                            class: "{BTN}",
                            onclick: move |_| game.write().mode = Mode::Bank,
                            "VISIT BANK"
                        }
                        button {
                            class: "{BTN}",
                            onclick: move |_| game.write().mode = Mode::Transfer,
                            "TRANSFER CARGO"
                        }
                        button {
                            class: "{BTN}",
                            onclick: move |_| game.write().visit_wu(),
                            "ELDER BROTHER WU"
                        }
                    }
                    button {
                        class: if at_hk { "crt-btn crt-btn-primary" } else { "crt-btn crt-btn-primary col-span-2" },
                        onclick: move |_| game.write().mode = Mode::Travel,
                        "SET SAIL ⛵"
                    }
                }
            }
        }
    }
}
