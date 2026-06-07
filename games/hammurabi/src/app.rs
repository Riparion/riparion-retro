//! App root: owns the one `Signal<Game>`, dispatches the current screen,
//! and persists every transition to localStorage.

use dioxus::prelude::*;

use crate::engine::scoring::HighScore;
use crate::engine::state::Mode;
use crate::storage;
use crate::ui::components::status_bar::{ChipStats, StatusBar};
use crate::ui::screens;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: MAIN_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1, viewport-fit=cover, user-scalable=no",
        }
        document::Title { "Hammurabi" }
        GameRoot {}
    }
}

#[component]
fn GameRoot() -> Element {
    let mut game = use_signal(storage::load_or_new);
    use_context_provider(|| game);
    // How many log lines the player has been shown; the year screen paces
    // this toward game.log.len(). A reload starts fully revealed.
    let revealed = use_signal(|| game.peek().log.len());
    use_context_provider(|| revealed);
    // Chip snapshot shown while a report is pacing (see StatusBar).
    let frozen = use_signal(|| ChipStats::of(&game.peek()));
    use_context_provider(|| frozen);

    // Persist mid-reign; finished terms clear the save and, when the reign
    // ended honourably, enter the hall of rulers exactly once.
    use_effect(move || {
        let snapshot = game.read();
        match snapshot.mode {
            Mode::Splash => {}
            Mode::Year => storage::save(&snapshot),
            Mode::GameOver => {
                storage::clear_save();
                if let Some(end) = snapshot.outcome.clone().filter(|e| !e.recorded) {
                    if end.score > 0 {
                        storage::record_score(HighScore {
                            score: end.score,
                            verdict: end.verdict.label().to_string(),
                            population: end.population,
                            acres: end.acres,
                        });
                    }
                    drop(snapshot);
                    if let Some(out) = game.write().outcome.as_mut() {
                        out.recorded = true;
                    }
                }
            }
        }
    });

    let mode = game.read().mode.clone();
    let in_game = matches!(mode, Mode::Year);

    rsx! {
        div { class: "crt flex flex-col h-[100dvh] overflow-hidden",
            if in_game {
                StatusBar {}
            }
            div { class: "flex-1 overflow-y-auto flex flex-col",
                match mode {
                    Mode::Splash => rsx! { screens::splash::Splash {} },
                    Mode::Year => rsx! { screens::year::Year {} },
                    Mode::GameOver => rsx! { screens::game_over::GameOver {} },
                }
            }
        }
    }
}
