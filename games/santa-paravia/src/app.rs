//! App root: owns the one `Signal<Game>`, dispatches the current screen, and
//! persists every transition to localStorage.

use dioxus::prelude::*;

use crate::engine::scoring::HighScore;
use crate::engine::state::Mode;
use crate::storage;
use crate::ui::components::status_bar::StatusBar;
use crate::ui::{screens, Overlay};

const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: MAIN_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1, viewport-fit=cover, user-scalable=no",
        }
        document::Title { "Santa Paravia and Fiumaccio" }
        GameRoot {}
    }
}

#[component]
fn GameRoot() -> Element {
    let mut game = use_signal(storage::load_or_new);
    use_context_provider(|| game);
    // Every load opens on the title screen, even when a save resumes a reign
    // (the splash then offers RESUME / NEW GAME). See NOTES.md "Start screen".
    let overlay = use_signal(|| Overlay::Splash);
    use_context_provider(|| overlay);

    // Persist every transition; a finished reign clears the save and posts a score.
    use_effect(move || {
        let snapshot = game.read();
        match snapshot.mode {
            Mode::Splash | Mode::NewGame => {}
            Mode::GameOver => {
                storage::clear_save();
                if let Some(end) = snapshot.outcome.clone().filter(|e| !e.recorded) {
                    storage::record_score(HighScore::from_end(&snapshot.state.name, &end));
                    drop(snapshot);
                    if let Some(out) = game.write().outcome.as_mut() {
                        out.recorded = true;
                    }
                }
            }
            _ => storage::save(&snapshot),
        }
    });

    let mode = game.read().mode;
    let overlay_now = overlay();
    let in_game = overlay_now == Overlay::Game
        && matches!(
            mode,
            Mode::YearReport
                | Mode::Market
                | Mode::Release
                | Mode::Tax
                | Mode::Purchases
                | Mode::Interaction
        );

    rsx! {
        div { class: "crt flex flex-col h-[100dvh] overflow-hidden",
            if in_game {
                StatusBar {}
            }
            div { class: "flex-1 overflow-y-auto flex flex-col",
                if overlay_now == Overlay::Help {
                    screens::help::Help {}
                } else if overlay_now == Overlay::Splash {
                    screens::splash::Splash {}
                } else {
                    match mode {
                        Mode::Splash => rsx! { screens::splash::Splash {} },
                        Mode::NewGame => rsx! { screens::new_game::NewGame {} },
                        Mode::YearReport => rsx! { screens::year_report::YearReport {} },
                        Mode::Market => rsx! { screens::market::Market {} },
                        Mode::Release => rsx! { screens::release::Release {} },
                        Mode::Tax => rsx! { screens::tax::Tax {} },
                        Mode::Purchases => rsx! { screens::purchases::Purchases {} },
                        Mode::Interaction => rsx! { screens::interaction_host::InteractionHost {} },
                        Mode::GameOver => rsx! { screens::game_over::GameOver {} },
                    }
                }
            }
        }
    }
}
