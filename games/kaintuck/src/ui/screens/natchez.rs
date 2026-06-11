//! Natchez Under-the-Hill: sell the cargo, break the boat up for lumber, risk a
//! night gambling, buy a horse, and set out up the Trace.

use dioxus::prelude::*;

use crate::engine::state::{fmt_money, Mode};
use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::components::stat_row::StatRow;
use retro_kit::theme::{BTN, PANEL, SCREEN};

const HORSE_PRICE: f64 = 12.0;

#[component]
pub fn Natchez() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut gambling = use_signal(|| false);

    let g = game.read();
    let s = &g.state;
    let cash = s.cash;
    let cargo_value = s.cargo_value();
    let has_cargo = s.cargo_units() > 0;
    let boat_lumber = s.boat.map(|b| b.lumber_value()).unwrap_or(0.0);
    let has_boat = s.boat.is_some();
    let has_horse = s.has_horse;
    drop(g);

    if gambling() {
        return rsx! {
            div { class: "flex-1 flex flex-col justify-center",
                NumberEntry {
                    prompt: "Lay your stake at the gambling house. Win and you double it; lose and it's gone — along with whatever a cutpurse can lift.".to_string(),
                    max: cash as i64,
                    confirm: "Lay it down".to_string(),
                    on_submit: move |amt: i64| {
                        gambling.set(false);
                        game.write().gamble(amt as f64);
                    },
                    on_cancel: move |_| gambling.set(false),
                }
            }
        };
    }

    rsx! {
        div { class: "{SCREEN} gap-3",
            div { class: "text-center",
                h2 { class: "text-lg tracking-widest", "NATCHEZ UNDER-THE-HILL" }
                p { class: "opacity-80 text-sm", "The wildest waterfront in America. End of the river — sell up and start the long walk home." }
            }
            div { class: "{PANEL} p-3 flex flex-col gap-1",
                StatRow { label: "Cash".to_string(), value: fmt_money(cash) }
                StatRow { label: "Cargo value".to_string(), value: fmt_money(cargo_value) }
                StatRow { label: "Boat as lumber".to_string(), value: fmt_money(boat_lumber) }
            }
            if has_cargo {
                MenuButton {
                    title: "Sell cargo".to_string(),
                    hint: "Top prices on the river — sell it all.".to_string(),
                    onclick: move |_| game.write().mode = Mode::Trade { buying: false },
                }
            }
            if has_boat {
                MenuButton {
                    title: "Sell the boat for lumber".to_string(),
                    hint: "She can't go back upstream. The wood is worth real money.".to_string(),
                    onclick: move |_| game.write().sell_boat(),
                }
            }
            MenuButton {
                title: "Spend a night Under-the-Hill".to_string(),
                hint: "Gambling, drinking, and thieves. Every dollar lost hurts the walk.".to_string(),
                onclick: move |_| gambling.set(true),
            }
            if !has_horse {
                MenuButton {
                    title: format!("Buy a horse — {}", fmt_money(HORSE_PRICE)),
                    hint: "Faster on the Trace, and the difference at the Duck River.".to_string(),
                    disabled: cash < HORSE_PRICE,
                    onclick: move |_| game.write().buy_horse(HORSE_PRICE),
                }
            }
            button {
                class: "{BTN} crt-btn-primary py-4 text-lg",
                onclick: move |_| game.write().set_out_on_trace(),
                "SET OUT ON THE TRACE ▸"
            }
        }
    }
}
