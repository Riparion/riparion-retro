//! Renders the head of the pending-interaction queue: paced messages and the
//! handful of yes/no prompts the river and trace throw at you.

use dioxus::prelude::*;

use crate::engine::interaction::{Interaction, Response};
use crate::engine::state::fmt_money;
use crate::engine::Game;
use retro_kit::components::decision::Decision;
use retro_kit::components::notice::Notice;

#[component]
pub fn InteractionHost() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let Some(head) = game.read().pending.front().cloned() else {
        game.write().resolve(Response::Acknowledge);
        return rsx! {};
    };

    match head {
        Interaction::Message { text, .. } => rsx! {
            Notice {
                message: text,
                on_ack: move |_| game.write().resolve(Response::Acknowledge),
            }
        },
        Interaction::CrewLeaves { pay } => yes_no(
            game,
            format!(
                "A hand wants {} to stay on, or he walks off here. Pay him?",
                fmt_money(pay)
            ),
            "LET HIM GO",
            "PAY HIM",
        ),
        Interaction::FerryToll { toll } => yes_no(
            game,
            format!(
                "A ferryman will pole you across the Duck River for {}. Pay, or ford it yourself?",
                fmt_money(toll)
            ),
            "FORD IT",
            "PAY TOLL",
        ),
    }
}

fn yes_no(mut game: Signal<Game>, prompt: String, left: &str, right: &str) -> Element {
    let left = left.to_string();
    let right = right.to_string();
    rsx! {
        Decision {
            prompt,
            left,
            on_left: move |_| game.write().resolve(Response::No),
            right,
            on_right: move |_| game.write().resolve(Response::Yes),
        }
    }
}
