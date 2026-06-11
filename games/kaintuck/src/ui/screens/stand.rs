//! A stand along the Trace: rest, buy a meal and provisions, maybe trade for a
//! horse, then press on.

use dioxus::prelude::*;

use crate::engine::state::{fmt_money, Stand as StandKind};
use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::format::fmt_num;
use retro_kit::theme::{PANEL, SCREEN_CENTERED};

const STAND_HORSE_PRICE: f64 = 14.0;

fn flavor(stand: StandKind) -> &'static str {
    match stand {
        StandKind::MountLocust => "The first inn north of Natchez. A meal of corn mush and a place on the porch.",
        StandKind::Choctaw => "Deep in Choctaw country, the forest closes in. Rest while you can.",
        StandKind::BuzzardRoost => "A prosperous Chickasaw stand. They keep travelers fed, safe — and coming back.",
        StandKind::TnDivide => "The Tennessee Valley Divide — the old border. Harpe country lies north.",
        StandKind::DuckRiver => "The Duck River crossing.",
    }
}

#[component]
pub fn Stand() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let stand = g.current_stand().unwrap_or(StandKind::MountLocust);
    let cash = g.state.cash;
    let provisions = g.state.provisions;
    let health = g.state.health;
    let has_horse = g.state.has_horse;
    drop(g);

    let can_trade_horse = matches!(stand, StandKind::BuzzardRoost) && !has_horse;

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            div { class: "{PANEL} p-3 text-center leading-snug",
                div { class: "text-lg tracking-widest", "{stand.label()}" }
                p { class: "text-sm opacity-80 mt-1", "{flavor(stand)}" }
                p { class: "text-xs opacity-60 mt-2",
                    "Provisions {fmt_num(provisions.max(0.0))} · {health.max(0.0) as i64}% strength · {fmt_money(cash)}"
                }
            }
            MenuButton {
                title: "Rest & resupply — $8".to_string(),
                hint: "+30 provisions, recover some strength.".to_string(),
                disabled: cash < 8.0,
                onclick: move |_| game.write().rest_and_resupply(),
            }
            if can_trade_horse {
                MenuButton {
                    title: format!("Trade for a horse — {}", fmt_money(STAND_HORSE_PRICE)),
                    hint: "The Chickasaw offer a horse. It earns its keep at the Duck River.".to_string(),
                    disabled: cash < STAND_HORSE_PRICE,
                    onclick: move |_| game.write().stand_buy_horse(STAND_HORSE_PRICE),
                }
            }
            MenuButton {
                title: "🥾 Press on ▸".to_string(),
                primary: true,
                onclick: move |_| game.write().leave_stand(),
            }
        }
    }
}
