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
        document::Title { "Fur Trader" }
        GameRoot {}
    }
}

#[component]
fn GameRoot() -> Element {
    let mut game = use_signal(storage::load_or_new);
    use_context_provider(|| game);
    // How many log lines the player has been shown; the results screen
    // paces this toward game.log.len(). A reload starts fully revealed.
    let revealed = use_signal(|| game.peek().log.len());
    use_context_provider(|| revealed);
    // Chip snapshot shown while a journal entry is pacing (see StatusBar).
    let frozen = use_signal(|| ChipStats::of(&game.peek()));
    use_context_provider(|| frozen);
    // Every load opens on the title screen, even when a save resumes an
    // expedition (the splash then offers RESUME / NEW GAME). See NOTES.md
    // "Start screen".
    let on_splash = use_signal(|| true);
    use_context_provider(|| on_splash);

    // Persist mid-run; finished runs clear the save and enter the hall of
    // traders exactly once.
    use_effect(move || {
        let snapshot = game.read();
        match snapshot.mode {
            Mode::Splash => {}
            Mode::Allocate | Mode::FortSelect | Mode::FortConfirm | Mode::Results => {
                storage::save(&snapshot)
            }
            Mode::GameOver => {
                storage::clear_save();
                if let Some(end) = snapshot.outcome.clone().filter(|e| !e.recorded) {
                    if end.score > 0 {
                        storage::record_score(HighScore {
                            score: end.score,
                            verdict: end.verdict.label().to_string(),
                            savings: end.savings.floor() as i64,
                            years: end.years,
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
    let show_splash = on_splash();
    let in_game = !show_splash
        && matches!(
            mode,
            Mode::Allocate | Mode::FortSelect | Mode::FortConfirm | Mode::Results
        );

    rsx! {
        div { class: "crt flex flex-col h-[100dvh] overflow-hidden",
            if in_game {
                StatusBar {}
            }
            div { class: "flex-1 overflow-y-auto flex flex-col",
                if show_splash {
                    screens::splash::Splash {}
                } else {
                    match mode {
                        Mode::Splash => rsx! { screens::splash::Splash {} },
                        Mode::Allocate => rsx! { screens::allocate::Allocate {} },
                        Mode::FortSelect => rsx! { screens::fort_select::FortSelect {} },
                        Mode::FortConfirm => rsx! { screens::fort_confirm::FortConfirm {} },
                        Mode::Results => rsx! { screens::results::Results {} },
                        Mode::GameOver => rsx! { screens::game_over::GameOver {} },
                    }
                }
            }
        }
    }
}
