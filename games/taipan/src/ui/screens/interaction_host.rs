//! Renders the head of the pending-interaction queue: paced messages and
//! yes/no/amount prompts from the arrival and voyage event chains.

use dioxus::prelude::*;

use crate::engine::interaction::{Interaction, Response};
use crate::engine::state::fmt_money;
use crate::engine::Game;
use retro_kit::components::decision::Decision;
use retro_kit::components::notice::Notice;
use retro_kit::components::number_entry::NumberEntry;

#[component]
pub fn InteractionHost() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let Some(head) = game.read().pending.front().cloned() else {
        // Defensive: an empty queue in Interaction mode just moves along.
        game.write().resolve(Response::Acknowledge);
        return rsx! {};
    };

    match head {
        Interaction::Message(text) => rsx! {
            Notice {
                message: text,
                on_ack: move |_| game.write().resolve(Response::Acknowledge),
            }
        },
        Interaction::LiYuenDonation { amount } => YesNo(
            game,
            format!(
                "Li Yuen asks {} in donation to the temple of Tin Hau, the Sea Goddess. Will you pay?",
                fmt_money(amount)
            ),
        ),
        Interaction::WuCoverDonation { amount, shortfall } => YesNo(
            game,
            format!(
                "Taipan, you do not have {} in cash! Do you want Elder Brother Wu to make up the difference of {}? (It will be added to your debt.)",
                fmt_money(amount),
                fmt_money(shortfall)
            ),
        ),
        Interaction::McHenryRepair { rate } => {
            let g = game.read();
            let damage = g.state.damage;
            let sw = g.state.seaworthiness();
            let full = (rate * damage + 1.0).floor();
            let max = g.state.cash.min(full) as i64;
            drop(g);
            rsx! {
                div { class: "flex-1 flex flex-col justify-center max-w-md w-full mx-auto",
                    NumberEntry {
                        prompt: format!(
                            "Taipan, McHenry from the Hong Kong shipyards has arrived. He says, 'I see ye've a wee bit of damage to yer ship — {}% seaworthy. 'Twill cost ye {} to fully repair. How much will ye spend?'",
                            sw,
                            fmt_money(full)
                        ),
                        max,
                        confirm: "Pay McHenry".to_string(),
                        on_submit: move |amount: i64| game.write().resolve(Response::Amount(amount as f64)),
                        on_cancel: move |_| game.write().resolve(Response::Amount(0.0)),
                    }
                }
            }
        }
        Interaction::WuVisitPrompt => YesNo(
            game,
            "Do you have business with Elder Brother Wu, the moneylender?".to_string(),
        ),
        Interaction::WuBailout { offer, repay } => YesNo(
            game,
            format!(
                "Elder Brother Wu is aware of your plight, Taipan. He is willing to loan you an additional {} if you will pay back {}. Are you willing? (Refuse, and the game is over!)",
                fmt_money(offer),
                fmt_money(repay)
            ),
        ),
        Interaction::ShipOffer { price, damaged } => YesNo(
            game,
            format!(
                "Do you wish to trade in your {}ship for one with 50 more capacity by paying an additional {}, Taipan?",
                if damaged { "damaged " } else { "" },
                fmt_money(price)
            ),
        ),
        Interaction::GunOffer { price } => YesNo(
            game,
            format!(
                "Do you wish to buy a ship's gun for {}, Taipan? (It takes 10 units of hold space.)",
                fmt_money(price)
            ),
        ),
    }
}

#[allow(non_snake_case)]
fn YesNo(mut game: Signal<Game>, prompt: String) -> Element {
    rsx! {
        Decision {
            prompt,
            left: "NO".to_string(),
            on_left: move |_| game.write().resolve(Response::No),
            right: "YES".to_string(),
            on_right: move |_| game.write().resolve(Response::Yes),
        }
    }
}
