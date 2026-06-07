//! Retirement or ruin: the final reckoning.

use dioxus::prelude::*;

use crate::engine::state::{fmt_money, Mode};
use crate::engine::Game;
use retro_kit::theme::{BTN, BTN_PRIMARY, PANEL};

#[component]
pub fn GameOver() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let Some(end) = g.outcome.clone() else {
        return rsx! {};
    };
    let firm = g.state.firm.clone();
    drop(g);

    let banner = if end.retired { "M I L L I O N A I R E !" } else { "G A M E   O V E R" };
    let advice = if end.score < 0 {
        Some("The crew has requested that you stay on shore for their safety!!")
    } else if end.score < 100 {
        Some("Have you considered a land-based job?")
    } else {
        None
    };

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-4 p-4 text-center",
            h1 { class: if end.retired { "splash-title" } else { "splash-title chip-danger" }, "{banner}" }
            p { class: "max-w-sm leading-snug", "{end.cause}" }
            div { class: "{PANEL} p-4 w-full max-w-sm text-left text-sm flex flex-col gap-1",
                Row { label: "Firm", value: firm }
                Row { label: "Net cash", value: fmt_money(end.net_worth) }
                Row { label: "Ship", value: format!("{} units, {} guns", end.capacity, end.guns) }
                Row {
                    label: "Time traded",
                    value: format!("{} year(s), {} month(s)", end.years, end.months),
                }
                Row { label: "Score", value: end.score.to_string() }
                Row { label: "Rating", value: end.rank.clone() }
            }
            if let Some(advice) = advice {
                p { class: "text-sm opacity-80 max-w-sm", "{advice}" }
            }
            div { class: "flex gap-2 w-full max-w-sm",
                button {
                    class: "{BTN} flex-1 py-3",
                    onclick: move |_| game.write().mode = Mode::Splash,
                    "TITLE"
                }
                button {
                    class: "{BTN_PRIMARY} flex-1 py-3",
                    onclick: move |_| game.write().mode = Mode::NewGame,
                    "PLAY AGAIN"
                }
            }
        }
    }
}

#[component]
fn Row(label: String, value: String) -> Element {
    rsx! {
        div { class: "flex justify-between gap-3",
            span { class: "chip-label", "{label}" }
            span { class: "text-right", "{value}" }
        }
    }
}
