//! A stand along the Trace: rest, buy a meal and provisions, maybe trade for a
//! horse, then press on. The status panel stays here; the options come from the
//! scenario, rendered through the generic set-piece menu.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::state::{fmt_money, Stand as StandKind};
use crate::engine::Game;
use crate::ui::components::set_piece_menu::SetPieceMenu;
use retro_kit::format::fmt_num;
use retro_kit::theme::{PANEL, SCREEN_CENTERED};

#[component]
pub fn Stand() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let stand = g.current_stand().unwrap_or(StandKind::MountLocust);
    let cash = g.state.cash;
    let provisions = g.state.provisions;
    let health = g.state.health;
    drop(g);

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            div { class: "{PANEL} p-3 text-center leading-snug",
                div { class: "text-lg tracking-widest", "{stand.label()}" }
                p { class: "text-sm opacity-80 mt-1", "{stand.flavor()}" }
                p { class: "text-xs opacity-60 mt-2",
                    "Provisions {fmt_num(provisions.max(0.0))} · {health.max(0.0) as i64}% strength · {fmt_money(cash)}"
                }
            }
            SetPieceMenu {
                options: scenario().menus.stand.options.as_slice(),
                onselect: move |(action, cost): (String, f64)| {
                    game.write().run_set_piece(&action, cost);
                },
            }
        }
    }
}
