//! Buy/sell: pick a good, then a quantity. Returns to whichever hub you came
//! from — the Pittsburgh boatyard, a river town, or Natchez.

use dioxus::prelude::*;

use crate::engine::state::{fmt_money, Mode, GOOD_NAMES, NATCHEZ, PITTSBURGH};
use crate::engine::Game;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::theme::{BTN, BTN_WIDE};

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
pub fn Trade(buying: bool) -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut good = use_signal(|| Option::<usize>::None);
    let back = hub_mode(game.read().state.town);

    if let Some(i) = good() {
        let g = game.read();
        let price = g.state.prices[i];
        let max = if buying { g.max_buy(i) } else { g.state.hold[i] };
        let verb = if buying { "load" } else { "sell" };
        let mut prompt = format!(
            "How much {} to {}? It's {} a unit.",
            GOOD_NAMES[i],
            verb,
            fmt_money(price)
        );
        if buying {
            prompt.push_str(&format!(" {} units of hold free.", g.state.free_hold().max(0)));
        }
        drop(g);
        return rsx! {
            div { class: "flex-1 flex flex-col justify-center",
                NumberEntry {
                    prompt,
                    max,
                    price: Some(price),
                    confirm: if buying { "Load".to_string() } else { "Sell".to_string() },
                    on_submit: move |qty| {
                        let mut g = game.write();
                        if buying {
                            g.buy(i, qty);
                        } else {
                            g.sell(i, qty);
                        }
                        g.mode = back.clone();
                    },
                    on_cancel: move |_| good.set(None),
                }
            }
        };
    }

    let g = game.read();
    let title = if buying { "Load what?" } else { "Sell what?" };
    let rows: Vec<(usize, String, String)> = (0..GOOD_NAMES.len())
        .map(|i| {
            let extra = if buying {
                format!("afford {}", g.max_buy(i))
            } else {
                format!("{} aboard", g.state.hold[i])
            };
            (i, fmt_money(g.state.prices[i]), extra)
        })
        .collect();
    drop(g);

    rsx! {
        div { class: "flex-1 flex flex-col gap-2 p-3 justify-center max-w-md w-full mx-auto",
            h2 { class: "text-center mb-2", "{title}" }
            for (i, price, extra) in rows {
                button {
                    key: "{i}",
                    class: "{BTN_WIDE} flex justify-between items-center py-3",
                    onclick: move |_| good.set(Some(i)),
                    span { "{GOOD_NAMES[i]}" }
                    span { class: "text-right text-sm opacity-80", "{price} · {extra}" }
                }
            }
            button {
                class: "{BTN} mt-3",
                onclick: move |_| game.write().mode = back.clone(),
                "Back"
            }
        }
    }
}
