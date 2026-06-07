//! The Hong Kong bank: deposits earn 0.5% per month at sea.

use dioxus::prelude::*;

use crate::engine::state::{fmt_money, Mode};
use crate::engine::Game;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::theme::{BTN, BTN_WIDE, PANEL};

#[component]
pub fn Bank() -> Element {
    let mut game = use_context::<Signal<Game>>();
    // Some(true) = depositing, Some(false) = withdrawing
    let mut action = use_signal(|| Option::<bool>::None);

    if let Some(depositing) = action() {
        let g = game.read();
        let max = if depositing { g.state.cash } else { g.state.bank } as i64;
        let prompt = format!(
            "How much will you {}, Taipan?",
            if depositing { "deposit" } else { "withdraw" }
        );
        drop(g);
        return rsx! {
            div { class: "flex-1 flex flex-col justify-center",
                NumberEntry {
                    prompt,
                    max,
                    confirm: if depositing { "Deposit".to_string() } else { "Withdraw".to_string() },
                    on_submit: move |amount| {
                        let mut g = game.write();
                        if depositing {
                            g.deposit(amount as f64);
                        } else {
                            g.withdraw(amount as f64);
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
    let bank = fmt_money(g.state.bank);
    let has_cash = g.state.cash >= 1.0;
    let has_bank = g.state.bank >= 1.0;
    drop(g);

    rsx! {
        div { class: "flex-1 flex flex-col gap-3 p-3 justify-center max-w-md w-full mx-auto",
            h2 { class: "chip-label text-center", "── HONG KONG BANK ──" }
            div { class: "{PANEL} p-3 text-center",
                p { "Cash in hand: {cash}" }
                p { "On deposit: {bank}" }
                p { class: "text-xs opacity-70 mt-1", "Deposits earn ½% each month at sea." }
            }
            button {
                class: "{BTN_WIDE} py-3",
                disabled: !has_cash,
                onclick: move |_| action.set(Some(true)),
                "DEPOSIT"
            }
            button {
                class: "{BTN_WIDE} py-3",
                disabled: !has_bank,
                onclick: move |_| action.set(Some(false)),
                "WITHDRAW"
            }
            button {
                class: "{BTN} mt-3",
                onclick: move |_| game.write().mode = Mode::Port,
                "Done"
            }
        }
    }
}
