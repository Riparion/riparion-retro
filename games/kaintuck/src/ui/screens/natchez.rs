//! Natchez Under-the-Hill: sell the cargo, break the boat up for lumber, risk a
//! night gambling, buy a horse, and set out up the Trace. The options come from
//! the scenario; the gambling stake entry is the one bit of screen state.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::state::{fmt_money, Mode};
use crate::engine::Game;
use crate::ui::components::set_piece_menu::SetPieceMenu;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::components::stat_row::StatRow;
use retro_kit::theme::{PANEL, SCREEN};

#[component]
pub fn Natchez() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut gambling = use_signal(|| false);

    let g = game.read();
    let s = &g.state;
    let cash = s.cash;
    // The realizable sale value (every good at its bid), not the mid-quote — this
    // is the money the player actually walks away with when they sell up.
    let cargo_value = g.cargo_sale_value();
    let boat_lumber = s.boat.map(|b| b.lumber_value()).unwrap_or(0.0);
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
            SetPieceMenu {
                options: scenario().menus.natchez.options.as_slice(),
                onselect: move |(action, cost): (String, f64)| {
                    match action.as_str() {
                        "sell-cargo" => game.write().mode = Mode::Trade { can_buy: false, can_sell: true },
                        "gamble" => gambling.set(true),
                        "moneylender" => game.write().mode = Mode::Moneylender,
                        _ => game.write().run_set_piece(&action, cost),
                    }
                },
            }
        }
    }
}
