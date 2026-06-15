//! Natchez Under-the-Hill: sell the cargo, break the boat up for lumber, sit in
//! at a card table (faro or vingt-et-un), buy a horse, and set out up the Trace.
//! The options come from the scenario; the two open-sheet flags (the main menu
//! and the saloon submenu) are the only screen state.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::state::{fmt_money, Mode};
use crate::engine::Game;
use crate::ui::components::set_piece_menu::{SetPieceMenu, Show};
use retro_kit::components::sheet::Sheet;
use retro_kit::components::stat_row::StatRow;
use retro_kit::theme::{ACTION_BAR, BTN, PANEL};

#[component]
pub fn Natchez() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut sheet_open = use_signal(|| false);
    let mut saloon_open = use_signal(|| false);

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

    // One dispatcher shared by every menu render (the visible primary "set out",
    // the secondary actions in the main sheet, and the saloon's card tables).
    // "saloon" swaps the main sheet for the saloon submenu; everything else
    // closes both sheets first. Sitting at a table carries the whole purse to it
    // as chips (the engine clamps to cash and escrows it); you control your risk
    // per hand and rise with whatever chips you leave with.
    let dispatch = move |(action, cost): (String, f64)| {
        if action == "saloon" {
            sheet_open.set(false);
            saloon_open.set(true);
            return;
        }
        sheet_open.set(false);
        saloon_open.set(false);
        match action.as_str() {
            "sell-cargo" => game.write().mode = Mode::Trade { can_buy: false, can_sell: true },
            "play-faro" => game.write().play_faro(cash),
            "play-vingt-un" => game.write().play_vingt_un(cash),
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
            Sheet {
                open: saloon_open(),
                on_close: move |_| saloon_open.set(false),
                title: "── THE SALOON ──".to_string(),
                div { class: "flex flex-col gap-2",
                    SetPieceMenu { options, onselect: dispatch, show: Show::Group("saloon") }
                }
            }
        }
    }
}
