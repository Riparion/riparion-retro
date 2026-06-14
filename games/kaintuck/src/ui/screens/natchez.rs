//! Natchez Under-the-Hill: sell the cargo, break the boat up for lumber, risk a
//! night gambling, buy a horse, and set out up the Trace. The options come from
//! the scenario; the gambling stake entry is the one bit of screen state.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::state::{fmt_money, Mode};
use crate::engine::Game;
use crate::ui::components::set_piece_menu::{SetPieceMenu, Show};
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::components::sheet::Sheet;
use retro_kit::components::stat_row::StatRow;
use retro_kit::theme::{ACTION_BAR, BTN, PANEL};

#[component]
pub fn Natchez() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut gambling = use_signal(|| false);
    let mut sheet_open = use_signal(|| false);

    let g = game.read();
    let s = &g.state;
    let cash = s.cash;
    // The realizable sale value (every good at its bid), not the mid-quote — this
    // is the money the player actually walks away with when they sell up.
    let cargo_value = g.cargo_sale_value();
    // What you'll actually be paid for her timbers — the damage-discounted
    // salvage, not the sound-hull lumber price (matches sell_boat / set_out).
    let boat_lumber = s.boat.map(|b| b.salvage_value()).unwrap_or(0.0);
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

    // One dispatcher shared by both menu renders (the visible primary "set out"
    // and the secondary actions tucked in the sheet); it closes the sheet first.
    let dispatch = move |(action, cost): (String, f64)| {
        sheet_open.set(false);
        match action.as_str() {
            "sell-cargo" => game.write().mode = Mode::Trade { can_buy: false, can_sell: true },
            "gamble" => gambling.set(true),
            "moneylender" => game.write().mode = Mode::Moneylender,
            _ => game.write().run_set_piece(&action, cost),
        }
    };
    let options = scenario().menus.natchez.options.as_slice();

    rsx! {
        div { class: "flex-1 flex flex-col",
            div { class: "flex-1 overflow-y-auto p-4 flex flex-col gap-3 max-w-md w-full mx-auto",
                div { class: "text-center",
                    h2 { class: "text-lg tracking-widest", "NATCHEZ UNDER-THE-HILL" }
                    p { class: "opacity-80 text-sm", "The wildest waterfront in America. End of the river — sell up and start the long walk home." }
                }
                div { class: "{PANEL} p-3 flex flex-col gap-1",
                    StatRow { label: "Cash".to_string(), value: fmt_money(cash) }
                    StatRow { label: "Cargo value".to_string(), value: fmt_money(cargo_value) }
                    StatRow { label: "Boat as lumber".to_string(), value: fmt_money(boat_lumber) }
                }
            }
            div { class: "{ACTION_BAR} flex flex-col gap-2",
                button {
                    class: "{BTN}",
                    onclick: move |_| sheet_open.set(true),
                    "MORE ▲"
                }
                SetPieceMenu { options, onselect: dispatch, show: Show::Primary }
            }
            Sheet {
                open: sheet_open(),
                on_close: move |_| sheet_open.set(false),
                title: "── UNDER-THE-HILL ──".to_string(),
                div { class: "flex flex-col gap-2",
                    SetPieceMenu { options, onselect: dispatch, show: Show::Secondary }
                }
            }
        }
    }
}
