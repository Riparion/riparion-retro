//! Buy/sell: pick a commodity, then a quantity.

use dioxus::prelude::*;

use crate::engine::state::{fmt_money, Mode, ITEM_NAMES};
use crate::engine::Game;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::theme::{BTN, BTN_WIDE};

#[component]
pub fn Trade(buying: bool) -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut item = use_signal(|| Option::<usize>::None);

    if let Some(i) = item() {
        let g = game.read();
        let price = g.state.prices[i];
        let max = if buying { g.max_buy(i) } else { g.state.hold[i] };
        let verb = if buying { "buy" } else { "sell" };
        let mut prompt = format!(
            "How much {} shall I {}, Taipan? It's {} per unit.",
            ITEM_NAMES[i],
            verb,
            fmt_money(price)
        );
        if buying {
            // Overloading is allowed (the original catches it at departure),
            // but the player deserves to see the hold space.
            prompt.push_str(&format!(" We have {} free hold space.", g.state.free_hold()));
        }
        drop(g);
        return rsx! {
            div { class: "flex-1 flex flex-col justify-center",
                NumberEntry {
                    prompt,
                    max,
                    price: Some(price),
                    confirm: if buying { "Buy".to_string() } else { "Sell".to_string() },
                    on_submit: move |qty| {
                        let mut g = game.write();
                        if buying {
                            g.buy(i, qty);
                        } else {
                            g.sell(i, qty);
                        }
                        g.mode = Mode::Port;
                    },
                    on_cancel: move |_| item.set(None),
                }
            }
        };
    }

    let g = game.read();
    let title = if buying { "What shall I buy, Taipan?" } else { "What shall I sell, Taipan?" };
    let rows: Vec<(usize, String, String)> = (0..4)
        .map(|i| {
            let extra = if buying {
                format!("can afford {}", g.max_buy(i))
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
                    onclick: move |_| item.set(Some(i)),
                    span { "{ITEM_NAMES[i]}" }
                    span { class: "text-right text-sm opacity-80", "{price} · {extra}" }
                }
            }
            button {
                class: "{BTN} mt-3",
                onclick: move |_| game.write().mode = Mode::Port,
                "Back"
            }
        }
    }
}
