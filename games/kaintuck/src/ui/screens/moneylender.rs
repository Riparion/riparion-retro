//! The moneylender at Cincinnati and Memphis, and at Natchez (to clear the debt
//! before the cash-out): borrow against the boat, or repay.

use dioxus::prelude::*;

use crate::engine::state::{fmt_money, Mode, NATCHEZ};
use crate::engine::Game;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::components::stat_row::StatRow;
use retro_kit::theme::{BTN, BTN_WIDE, PANEL, SCREEN_CENTERED};

#[derive(Clone, Copy, PartialEq)]
enum Sub {
    Menu,
    Borrow,
    Repay,
}

#[component]
pub fn Moneylender() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut sub = use_signal(|| Sub::Menu);

    let g = game.read();
    let cash = g.state.cash;
    let debt = g.state.debt;
    let max_borrow = g.max_borrow();
    let interest_pct = g.interest_rate() * 100.0;
    // The moneylender sits in a river-town landing, and at Natchez — go back to
    // whichever hub the player came from.
    let back = if g.state.town == NATCHEZ {
        Mode::Natchez
    } else {
        Mode::Town
    };
    drop(g);

    match sub() {
        Sub::Borrow => rsx! {
            div { class: "flex-1 flex flex-col justify-center",
                NumberEntry {
                    prompt: format!("Borrow how much? You can draw up to {}.", fmt_money(max_borrow)),
                    max: max_borrow as i64,
                    confirm: "Borrow".to_string(),
                    on_submit: move |amt: i64| {
                        game.write().borrow(amt as f64);
                        sub.set(Sub::Menu);
                    },
                    on_cancel: move |_| sub.set(Sub::Menu),
                }
            }
        },
        Sub::Repay => rsx! {
            div { class: "flex-1 flex flex-col justify-center",
                NumberEntry {
                    prompt: format!("Repay how much? You owe {}.", fmt_money(debt)),
                    max: cash.min(debt) as i64,
                    confirm: "Repay".to_string(),
                    on_submit: move |amt: i64| {
                        game.write().repay(amt as f64);
                        sub.set(Sub::Menu);
                    },
                    on_cancel: move |_| sub.set(Sub::Menu),
                }
            }
        },
        Sub::Menu => rsx! {
            div { class: "{SCREEN_CENTERED} gap-3",
                h2 { class: "text-center text-lg tracking-widest", "THE MONEYLENDER" }
                div { class: "{PANEL} p-3 flex flex-col gap-1",
                    StatRow { label: "Cash".to_string(), value: fmt_money(cash) }
                    StatRow { label: "Owed".to_string(), value: fmt_money(debt) }
                    StatRow { label: "Can borrow".to_string(), value: fmt_money(max_borrow) }
                }
                p { class: "text-xs opacity-60 text-center", "Interest runs {interest_pct:.1}% a leg — your standing sets the rate. Debt comes off your earnings at Natchez." }
                button { class: "{BTN_WIDE} py-3", onclick: move |_| sub.set(Sub::Borrow), "BORROW" }
                button { class: "{BTN_WIDE} py-3", disabled: debt <= 0.0, onclick: move |_| sub.set(Sub::Repay), "REPAY" }
                button { class: "{BTN}", onclick: move |_| game.write().mode = back.clone(), "Back to the landing" }
            }
        },
    }
}
