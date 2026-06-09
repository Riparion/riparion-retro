//! Buy or sell land — one deal per year, in grain.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::components::error_banner::ErrorBanner;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::components::seg_button::SegButton;
use retro_kit::theme::BTN;

#[component]
pub fn Land() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let buy_price = g.land_buy_price;
    let sell_price = g.land_sell_price();
    let grain = g.state.n_g;
    let good = g.state.good_land();
    let err = g.input_error.clone();
    drop(g);

    let mut selling = use_signal(|| false);
    let is_selling = selling();

    let max_buy = if buy_price > 0 { grain / buy_price } else { 0 };
    let sell_cap = if sell_price > 0 {
        good.min(4000 / sell_price)
    } else {
        good
    };
    let max = if is_selling { sell_cap } else { max_buy };

    let prompt = if is_selling {
        format!("Sell land at {sell_price} HL per HA. You hold {good} HA of good land.")
    } else {
        format!("Buy land at {buy_price} HL per HA. You can afford {max_buy} HA.")
    };
    let price = if is_selling {
        sell_price as f64
    } else {
        buy_price as f64
    };
    let confirm = if is_selling { "SELL" } else { "BUY" };

    rsx! {
        div { class: "flex-1 flex flex-col justify-center gap-2",
            div { class: "flex gap-2 mx-3",
                SegButton { label: "BUY", active: !is_selling, onclick: move |_| selling.set(false) }
                SegButton { label: "SELL", active: is_selling, onclick: move |_| selling.set(true) }
            }
            ErrorBanner { message: err }
            NumberEntry {
                prompt,
                max,
                price: Some(price),
                preview: Callback::new(move |v: i64| {
                    if is_selling {
                        format!("yields {} HL of grain", v * sell_price)
                    } else {
                        format!("costs {} HL of grain", v * buy_price)
                    }
                }),
                confirm: confirm.to_string(),
                on_submit: move |v: i64| {
                    let mut w = game.write();
                    let _ = if selling() { w.submit_sell_land(v) } else { w.submit_buy_land(v) };
                },
            }
            button {
                class: "{BTN} mx-3",
                onclick: move |_| {
                    let _ = game.write().submit_buy_land(0);
                },
                "No land deal this year"
            }
        }
    }
}
