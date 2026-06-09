//! Renders the head of the pending queue: paced narration and the in-stride
//! decisions of the King's demands and the chaos of war.

use dioxus::prelude::*;

use crate::engine::interaction::{Interaction, Response};
use crate::engine::state::Mode;
use crate::engine::Game;
use retro_kit::components::decision::Decision;
use retro_kit::components::notice::Notice;
use retro_kit::components::number_entry::NumberEntry;

#[component]
pub fn InteractionHost() -> Element {
    let mut game = use_context::<Signal<Game>>();
    // Recover from an inconsistent (Interaction mode, empty queue) state *after*
    // render rather than mutating during it. This never fires in normal play.
    use_effect(move || {
        let g = game.read();
        if g.mode == Mode::Interaction && g.pending.is_empty() {
            drop(g);
            game.write().resolve(Response::Acknowledge);
        }
    });
    let Some(head) = game.read().pending.front().cloned() else {
        return rsx! {};
    };

    match head {
        Interaction::Message(text) => rsx! {
            Notice {
                message: text,
                on_ack: move |_| game.write().resolve(Response::Acknowledge),
            }
        },
        Interaction::DoubleTax => decision(
            game,
            "The King demands twice the royal tax this year, hoping to provoke you. Will you pay?"
                .to_string(),
            "REFUSE",
            Response::No,
            "PAY",
            Response::Yes,
        ),
        Interaction::KingLevy { peasants, grain } => decision(
            game,
            format!(
                "The High King requires {peasants} peasants for his estates and mines. Supply them, or pay {grain} HL of grain instead?"
            ),
            format!("Pay {grain} HL"),
            Response::No,
            format!("Send {peasants}"),
            Response::Yes,
        ),
        Interaction::WarAttack => decision(
            game,
            "A nearby Duke threatens war! Will you strike first?".to_string(),
            "HOLD",
            Response::No,
            "STRIKE FIRST",
            Response::Yes,
        ),
        Interaction::WarMercenary { max, price } => {
            let cash_grain = game.read().state.n_g.max(0);
            rsx! {
                div { class: "flex-1 flex flex-col justify-center max-w-md w-full mx-auto",
                    NumberEntry {
                        prompt: format!(
                            "How many mercenaries will you hire for the battle? They cost {price} HL of grain each (up to {max}). You hold {cash_grain} HL."
                        ),
                        max,
                        preview: Callback::new(move |v: i64| format!("costs {} HL of grain", v * price)),
                        confirm: "TO BATTLE".to_string(),
                        on_submit: move |v: i64| game.write().resolve(Response::Amount(v)),
                    }
                }
            }
        }
    }
}

/// Wire retro-kit's [`Decision`] panel to the game's response queue, picking the
/// labels so each prompt reads naturally (Pay/Refuse, Send/Pay, Strike/Hold).
fn decision(
    mut game: Signal<Game>,
    prompt: String,
    left_label: impl Into<String>,
    left: Response,
    right_label: impl Into<String>,
    right: Response,
) -> Element {
    rsx! {
        Decision {
            prompt,
            left: left_label.into(),
            on_left: move |_| game.write().resolve(left),
            right: right_label.into(),
            on_right: move |_| game.write().resolve(right),
        }
    }
}
