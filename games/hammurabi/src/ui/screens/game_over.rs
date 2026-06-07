//! The judgment of history: the final reckoning.

use dioxus::prelude::*;

use crate::engine::scoring::Verdict;
use crate::engine::state::Mode;
use crate::engine::Game;
use crate::ui::components::status_bar::ChipStats;
use retro_kit::components::stat_row::StatRow;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN, BTN_PRIMARY, PANEL};

fn judgment(end: &crate::engine::scoring::EndGame) -> (&'static str, String) {
    match end.verdict {
        Verdict::Fantastic => (
            "A FANTASTIC PERFORMANCE!!!",
            "Charlemange, Disraeli, and Jefferson combined could not have done better!"
                .to_string(),
        ),
        Verdict::NotTooBad => (
            "NOT TOO BAD AT ALL",
            format!(
                "Your performance could have been somewhat better, but really wasn't \
                 too bad at all. {} people would dearly like to see you assassinated, \
                 but we all have our trivial problems.",
                group_thousands(end.haters),
            ),
        ),
        Verdict::HeavyHanded => (
            "AN UNPLEASANT RULER",
            "Your heavy-handed performance smacks of Nero and Ivan IV. The people \
             (remaining) find you an unpleasant ruler, and, frankly, hate your guts!!"
                .to_string(),
        ),
        Verdict::Fink => (
            "NATIONAL FINK!!!!",
            "Due to this extreme mismanagement you have not only been impeached and \
             thrown out of office, but you have also been declared NATIONAL FINK!!!!"
                .to_string(),
        ),
        Verdict::Impeached => (
            "NATIONAL FINK!!!!",
            format!(
                "You starved {} people in one year!!! Due to this extreme mismanagement \
                 you have not only been impeached and thrown out of office, but you \
                 have also been declared NATIONAL FINK!!!!",
                group_thousands(end.starved_final_year),
            ),
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

    let honourable = end.verdict.honourable();
    let banner = spaced(if honourable { "SO LONG FOR NOW" } else { "THROWN OUT" });
    let (headline, detail) = judgment(&end);

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-4 p-4 text-center",
            h1 { class: if honourable { "splash-title endgame-banner" } else { "splash-title endgame-banner chip-danger" },
                "{banner}"
            }
            p { class: "max-w-sm leading-snug font-bold", "{headline}" }
            p { class: "max-w-sm leading-snug text-sm opacity-80", "{detail}" }
            div { class: "{PANEL} p-4 w-full max-w-sm text-left text-sm flex flex-col gap-1",
                StatRow { label: "Years in office", value: end.years.to_string() }
                StatRow { label: "Final population", value: group_thousands(end.population) }
                StatRow { label: "Acres owned", value: group_thousands(end.acres) }
                StatRow {
                    label: "Acres per person",
                    value: format!("{:.1} (started with 10)", end.acres_per_person),
                }
                StatRow { label: "People starved", value: group_thousands(end.total_starved) }
                StatRow {
                    label: "Starved per year, avg",
                    value: format!("{:.1}%", end.avg_starved_pct),
                }
                StatRow { label: "Score", value: end.score.to_string() }
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
                    "RULE AGAIN"
                }
            }
        }
    }
}
