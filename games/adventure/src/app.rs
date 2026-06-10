//! App root: owns the one `Signal<Game>`, dispatches the current screen, and
//! persists every transition to localStorage.

use dioxus::prelude::*;

use crate::engine::Mode;
use crate::storage::{self, HighScore};
use crate::ui::components::status_bar::StatusBar;
use crate::ui::screens;

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
        document::Title { "Colossal Cave Adventure" }
        GameRoot {}
    }
}

#[component]
fn GameRoot() -> Element {
    let mut game = use_signal(storage::load_or_new);
    use_context_provider(|| game);
    // Every load opens on the title screen, even when a save can be resumed.
    let on_splash = use_signal(|| true);
    use_context_provider(|| on_splash);

    // Persist every transition; a finished adventure clears the save and records
    // its score exactly once.
    use_effect(move || {
        let snapshot = game.read();
        match snapshot.mode {
            Mode::Splash => {}
            Mode::GameOver => {
                storage::clear_save();
                if !snapshot.score_recorded {
                    let (score, _) = snapshot.final_score();
                    let entry = HighScore {
                        score,
                        turns: snapshot.turn_count(),
                        closed: snapshot.is_cave_closed(),
                    };
                    storage::record_score(entry);
                    drop(snapshot);
                    game.write().score_recorded = true;
                }
            }
            Mode::Playing => storage::save(&snapshot),
        }
    });

    let mode = game.read().mode;
    let show_splash = on_splash();

    rsx! {
        div { class: "crt flex flex-col h-[100dvh] overflow-hidden",
            if show_splash {
                div { class: "flex-1 overflow-y-auto flex flex-col", screens::splash::Splash {} }
            } else {
                match mode {
                    Mode::Splash => rsx! {
                        div { class: "flex-1 overflow-y-auto flex flex-col", screens::splash::Splash {} }
                    },
                    Mode::GameOver => rsx! {
                        div { class: "flex-1 overflow-y-auto flex flex-col", screens::game_over::GameOver {} }
                    },
                    Mode::Playing => rsx! {
                        StatusBar {}
                        screens::play::Play {}
                    },
                }
            }
        }
    }
}
