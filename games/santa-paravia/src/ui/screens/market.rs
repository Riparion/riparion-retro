//! The market: buy and sell grain and land, as many deals as you like, then
//! move on to feeding the city.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::components::error_banner::ErrorBanner;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::components::seg_button::SegButton;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN_PRIMARY, PANEL};

/// Which of the four deals the player is composing.
#[derive(Clone, Copy, PartialEq)]
enum Deal {
    BuyGrain,
    SellGrain,
    BuyLand,
    SellLand,
}

#[component]
pub fn Market() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let treasury = s.treasury;
    let grain = s.grain;
    let land = s.land;
    let grain_price = s.grain_price;
    let land_price = s.land_price;
    let err = g.input_error.clone();
    drop(g);

    let mut deal = use_signal(|| Deal::BuyGrain);
    let cur = deal();

    // Per-deal bounds and the live preview of florins moved.
    let (max, prompt, confirm): (i64, String, &str) = match cur {
        Deal::BuyGrain => {
            let m = if grain_price > 0 {
                (treasury.max(0) as i128 * 1000 / grain_price as i128) as i64
            } else {
                treasury.max(0)
            };
            (
                m,
                format!("Buy grain at {grain_price} fl per 1000 steres. Treasury {} fl.", group_thousands(treasury)),
                "BUY GRAIN",
            )
        }
        Deal::SellGrain => (
            grain,
            format!("Sell grain at {grain_price} fl per 1000 steres. Reserve {} st.", group_thousands(grain)),
            "SELL GRAIN",
        ),
        Deal::BuyLand => {
            let m = if land_price >= 1.0 {
                (treasury.max(0) as f64 / land_price) as i64
            } else {
                treasury.max(0)
            };
            (
                m,
                format!("Buy land at {land_price:.2} fl per hectare. Treasury {} fl.", group_thousands(treasury)),
                "BUY LAND",
            )
        }
        Deal::SellLand => (
            (land - 5000).max(0),
            format!("Sell land at {land_price:.2} fl per hectare. You may keep at most {} ha.", group_thousands(land - 5000)),
            "SELL LAND",
        ),
    };

    let preview = Callback::new(move |v: i64| match cur {
        Deal::BuyGrain => format!("costs {} fl", group_thousands(v * grain_price / 1000)),
        Deal::SellGrain => format!("yields {} fl", group_thousands(v * grain_price / 1000)),
        Deal::BuyLand => format!("costs {} fl", group_thousands((v as f64 * land_price) as i64)),
        Deal::SellLand => format!("yields {} fl", group_thousands((v as f64 * land_price) as i64)),
    });

    rsx! {
        div { class: "flex-1 flex flex-col justify-center gap-2",
            div { class: "grid grid-cols-2 gap-2 mx-3",
                SegButton { label: "Buy grain", active: cur == Deal::BuyGrain, onclick: move |_| deal.set(Deal::BuyGrain) }
                SegButton { label: "Sell grain", active: cur == Deal::SellGrain, onclick: move |_| deal.set(Deal::SellGrain) }
                SegButton { label: "Buy land", active: cur == Deal::BuyLand, onclick: move |_| deal.set(Deal::BuyLand) }
                SegButton { label: "Sell land", active: cur == Deal::SellLand, onclick: move |_| deal.set(Deal::SellLand) }
            }
            ErrorBanner { message: err }
            NumberEntry {
                prompt,
                max,
                preview,
                confirm: confirm.to_string(),
                on_submit: move |v: i64| {
                    let mut w = game.write();
                    let _ = match cur {
                        Deal::BuyGrain => w.buy_grain(v),
                        Deal::SellGrain => w.sell_grain(v),
                        Deal::BuyLand => w.buy_land(v),
                        Deal::SellLand => w.sell_land(v),
                    };
                },
            }
            div { class: "{PANEL} mx-3 p-2 text-center text-sm opacity-80",
                "Reserve {group_thousands(grain)} st · Land {group_thousands(land)} ha"
            }
            button {
                class: "{BTN_PRIMARY} mx-3 py-3",
                onclick: move |_| game.write().done_market(),
                "DONE — FEED THE CITY ▸"
            }
        }
    }
}
