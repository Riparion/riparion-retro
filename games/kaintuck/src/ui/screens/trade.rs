//! The market: one actionable table. Tap a good to open inline quantity entry
//! and buy and/or sell it in place — no drilling into separate screens, no
//! bouncing back to the hub between trades. `can_buy`/`can_sell` gate the
//! columns and actions: the Pittsburgh boatyard buys only, Natchez sells only,
//! river towns do both. "Done" returns to whichever hub you came from.

use dioxus::prelude::*;

use crate::engine::state::{fmt_money, Mode, GOOD_NAMES, NATCHEZ, PITTSBURGH};
use crate::engine::Game;
use retro_kit::theme::{BTN, BTN_PRIMARY, PANEL};

fn hub_mode(town: usize) -> Mode {
    if town == PITTSBURGH {
        Mode::Pittsburgh
    } else if town == NATCHEZ {
        Mode::Natchez
    } else {
        Mode::Town
    }
}

#[component]
pub fn Trade(can_buy: bool, can_sell: bool) -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut open = use_signal(|| Option::<usize>::None);
    let mut qty = use_signal(String::new);
    // A fresh row starts with an empty quantity, never the previous good's.
    use_effect(move || {
        open.read();
        qty.set(String::new());
    });

    let back = hub_mode(game.read().state.town);
    let title = match (can_buy, can_sell) {
        (true, false) => "BUY CARGO",
        (false, true) => "SELL CARGO",
        _ => "THE MARKET",
    };

    let g = game.read();
    let s = &g.state;
    let cash = s.cash;
    let cargo = s.cargo_units();
    let cap = s.capacity();
    // Snapshot every good's quote, ceiling, and aboard count for this render;
    // the component re-renders after each trade, so these stay live.
    let rows: Vec<(usize, f64, f64, i64, i64)> = (0..GOOD_NAMES.len())
        .map(|i| (i, g.buy_price(i), g.sell_price(i), g.max_buy(i), s.hold[i]))
        .collect();
    drop(g);

    // The quantity typed into the open row, clamped per action below.
    let typed = qty.read().parse::<i64>().unwrap_or(0).max(0);

    rsx! {
        div { class: "flex-1 flex flex-col",
            div { class: "p-3 text-center",
                h2 { class: "text-lg tracking-widest", "{title}" }
                p { class: "opacity-80 text-sm",
                    "Cash {fmt_money(cash)} · hold {cargo}/{cap}"
                }
            }
            div { class: "px-3 flex items-center gap-2 chip-label",
                span { class: "flex-1", "CARGO" }
                if can_buy {
                    span { class: "w-16 text-right", "BUY" }
                }
                if can_sell {
                    span { class: "w-16 text-right", "SELL" }
                }
                span { class: "w-14 text-right", "ABRD" }
            }
            div { class: "flex-1 overflow-y-auto px-3 pt-1 flex flex-col gap-2",
                for (i, buy_price, sell_price, max_buy, aboard) in rows {
                    div { key: "{i}", class: "{PANEL} overflow-hidden",
                        button {
                            class: "w-full flex items-center gap-2 px-3 py-3 text-left",
                            onclick: move |_| {
                                open.set(if open() == Some(i) { None } else { Some(i) });
                            },
                            span { class: "flex-1", "{GOOD_NAMES[i]}" }
                            if can_buy {
                                span { class: "w-16 text-right text-sm opacity-80", "{fmt_money(buy_price)}" }
                            }
                            if can_sell {
                                span { class: "w-16 text-right text-sm opacity-80", "{fmt_money(sell_price)}" }
                            }
                            span { class: "w-14 text-right text-sm",
                                if aboard > 0 { "{aboard} ▾" } else { "▾" }
                            }
                        }
                        if open() == Some(i) {
                            {
                                let buy_qty = typed.min(max_buy);
                                let sell_qty = typed.min(aboard);
                                // "All" fills the larger ceiling; each action clamps
                                // to its own (engine re-clamps regardless).
                                let all = if can_buy && can_sell {
                                    max_buy.max(aboard)
                                } else if can_buy {
                                    max_buy
                                } else {
                                    aboard
                                };
                                rsx! {
                                    div { class: "px-3 pb-3 pt-1 flex flex-col gap-2",
                                        div { class: "flex items-center gap-2",
                                            input {
                                                class: "crt-input flex-1",
                                                r#type: "number",
                                                inputmode: "numeric",
                                                pattern: "[0-9]*",
                                                min: "0",
                                                placeholder: "0",
                                                value: "{qty}",
                                                autofocus: true,
                                                oninput: move |e| qty.set(e.value()),
                                            }
                                            button {
                                                class: "{BTN}",
                                                onclick: move |_| qty.set(all.to_string()),
                                                "All ({all})"
                                            }
                                        }
                                        div { class: "flex gap-2",
                                            if can_buy {
                                                button {
                                                    class: "crt-btn crt-btn-primary flex-1",
                                                    disabled: max_buy == 0,
                                                    onclick: move |_| {
                                                        game.write().buy(i, typed);
                                                        qty.set(String::new());
                                                    },
                                                    "Buy {fmt_money(buy_price * buy_qty as f64)}"
                                                }
                                            }
                                            if can_sell {
                                                button {
                                                    class: "crt-btn crt-btn-primary flex-1",
                                                    disabled: aboard == 0,
                                                    onclick: move |_| {
                                                        game.write().sell(i, typed);
                                                        qty.set(String::new());
                                                    },
                                                    "Sell {fmt_money(sell_price * sell_qty as f64)}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "p-3",
                button {
                    class: "{BTN_PRIMARY}",
                    onclick: move |_| game.write().mode = back.clone(),
                    "Done ▸"
                }
            }
        }
    }
}
