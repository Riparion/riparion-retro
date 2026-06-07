//! The final reckoning: how the run ended and what it banked.

use dioxus::prelude::*;

use crate::engine::scoring::Verdict;
use crate::engine::state::Mode;
use crate::engine::{fmt_dollars, Game};
use crate::ui::components::status_bar::ChipStats;
use retro_kit::components::stat_row::StatRow;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN, BTN_PRIMARY, PANEL};

fn reckoning(end: &crate::engine::scoring::EndGame) -> (&'static str, String) {
    match end.verdict {
        Verdict::Retired => (
            "SO LONG FOR NOW",
            format!(
                "You hang up the paddle after {} {} on the river and retire on your savings.",
                end.years,
                if end.years == 1 { "year" } else { "years" },
            ),
        ),
        Verdict::Killed => (
            "THE RIVER TAKES YOU",
            "You were attacked by a party of Iroquois on the way to Fort New York. \
             All people in your trading group were killed. This ends the game."
                .to_string(),
        ),
        Verdict::Bankrupt => (
            "RUINED",
            "You cannot raise the $105.00 it takes to outfit even the cheapest \
             expedition. Your trading days are over."
                .to_string(),
        ),
    }
}

/// Letter-space a banner with non-breaking gaps so it only wraps between
/// words (a plain "S O   L O N G" wraps mid-word on narrow screens).
fn spaced(words: &str) -> String {
    words
        .split_whitespace()
        .map(|w| w.chars().map(String::from).collect::<Vec<_>>().join("\u{a0}"))
        .collect::<Vec<_>>()
        .join("  ")
}

#[component]
pub fn GameOver() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut revealed = use_context::<Signal<usize>>();
    let mut frozen = use_context::<Signal<ChipStats>>();
    let g = game.read();
    let Some(end) = g.outcome.clone() else {
        return rsx! {};
    };
    drop(g);

    let survived = end.verdict == Verdict::Retired;
    let banner = spaced(reckoning(&end).0);
    let detail = reckoning(&end).1;

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-4 p-4 text-center",
            h1 { class: if survived { "splash-title endgame-banner" } else { "splash-title endgame-banner chip-danger" },
                "{banner}"
            }
            p { class: "max-w-sm leading-snug text-sm opacity-80", "{detail}" }
            div { class: "{PANEL} p-4 w-full max-w-sm text-left text-sm flex flex-col gap-1",
                StatRow { label: "Verdict", value: end.verdict.label().to_string() }
                StatRow { label: "Years traded", value: end.years.to_string() }
                StatRow { label: "Final savings", value: fmt_dollars(end.savings) }
                StatRow { label: "Score", value: format!("${}", group_thousands(end.score)) }
            }
            div { class: "flex gap-2 w-full max-w-sm",
                button {
                    class: "{BTN} flex-1 py-3",
                    onclick: move |_| game.write().mode = Mode::Splash,
                    "TITLE"
                }
                button {
                    class: "{BTN_PRIMARY} flex-1 py-3",
                    onclick: move |_| {
                        revealed.set(0);
                        game.write().start();
                        frozen.set(ChipStats::of(&game.peek()));
                    },
                    "TRADE AGAIN"
                }
            }
        }
    }
}
