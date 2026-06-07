//! Elder Brother Wu, the moneylender. Borrow at most twice your cash;
//! his interest compounds 10% a month, and his patience has limits.

use dioxus::prelude::*;

use crate::engine::state::fmt_money;
use crate::engine::Game;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::theme::{BTN, BTN_WIDE, PANEL};

#[component]
pub fn Wu() -> Element {
    let mut game = use_context::<Signal<Game>>();
    // Some(true) = repaying, Some(false) = borrowing
    let mut action = use_signal(|| Option::<bool>::None);

    if let Some(repaying) = action() {
        let g = game.read();
        let max = if repaying {
            g.state.cash.min(g.state.debt)
        } else {
            g.max_borrow()
        } as i64;
        let prompt = if repaying {
            "How much will you repay him, Taipan?".to_string()
        } else {
            format!(
                "How much do you wish to borrow, Taipan? (He will loan up to {}.)",
                fmt_money(g.max_borrow())
            )
        };
        drop(g);
        return rsx! {
            div { class: "flex-1 flex flex-col justify-center",
                NumberEntry {
                    prompt,
                    max,
                    confirm: if repaying { "Repay".to_string() } else { "Borrow".to_string() },
                    on_submit: move |amount| {
                        let mut g = game.write();
                        if repaying {
                            g.repay(amount as f64);
                        } else {
                            g.borrow(amount as f64);
                        }
                        action.set(None);
                    },
                    on_cancel: move |_| action.set(None),
                }
            }
        };
    }

    let g = game.read();
    let cash = fmt_money(g.state.cash);
    let debt = fmt_money(g.state.debt);
    let can_repay = g.state.cash >= 1.0 && g.state.debt >= 1.0;
    let can_borrow = g.state.cash >= 1.0;
    drop(g);

    rsx! {
        div { class: "flex-1 flex flex-col gap-3 p-3 justify-center max-w-md w-full mx-auto",
            h2 { class: "chip-label text-center", "── ELDER BROTHER WU ──" }
            div { class: "{PANEL} p-3 text-center",
                p { "Cash in hand: {cash}" }
                p { "Owed to Wu: {debt}" }
                p { class: "text-xs opacity-70 mt-1",
                    "Wu's debt grows 10% each month at sea. He remembers, Taipan."
                }
            }
            button {
                class: "{BTN_WIDE} py-3",
                disabled: !can_repay,
                onclick: move |_| action.set(Some(true)),
                "REPAY WU"
            }
            button {
                class: "{BTN_WIDE} py-3",
                disabled: !can_borrow,
                onclick: move |_| action.set(Some(false)),
                "BORROW FROM WU"
            }
            button {
                class: "{BTN} mt-3",
                onclick: move |_| game.write().leave_wu(),
                "Take leave of Wu"
            }
        }
    }
}
