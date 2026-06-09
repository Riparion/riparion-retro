//! App root: owns the one `Signal<Game>`, dispatches the current screen,
//! and persists every transition to localStorage.

use dioxus::prelude::*;

use crate::engine::scoring::HighScore;
use crate::engine::state::Mode;
use crate::storage;
use crate::ui::components::status_bar::StatusBar;
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
        document::Title { "The Oregon Trail" }
        GameRoot {}
    }
}

#[component]
fn GameRoot() -> Element {
    let mut game = use_signal(storage::load_or_new);
    use_context_provider(|| game);
    // Every load opens on the title screen, even when a save resumes a journey
    // (the splash then offers RESUME / NEW GAME). See NOTES.md "Start screen".
    let on_splash = use_signal(|| true);
    use_context_provider(|| on_splash);

    // Persist every transition; finished journeys clear the save and post a score.
    use_effect(move || {
        let snapshot = game.read();
        match snapshot.mode {
            Mode::Splash | Mode::NewGame => {}
            Mode::GameOver => {
                storage::clear_save();
                if let Some(end) = snapshot.outcome.clone().filter(|e| !e.recorded) {
                    storage::record_score(HighScore::from_end(&snapshot.state.party, &end));
                    drop(snapshot);
                    if let Some(out) = game.write().outcome.as_mut() {
                        out.recorded = true;
                    }
                }
            }
            _ => storage::save(&snapshot),
        }
    });

    let mode = game.read().mode.clone();
    let show_splash = on_splash();
    let in_game = !show_splash
        && !matches!(
            mode,
            Mode::Splash | Mode::NewGame | Mode::Outfit | Mode::GameOver
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
                    Mode::NewGame => rsx! { screens::new_game::NewGame {} },
                    Mode::Outfit => rsx! { screens::outfit::Outfit {} },
                    Mode::Trail => rsx! { screens::trail::Trail {} },
                    Mode::Fort => rsx! { screens::fort::Fort {} },
                    Mode::Eat => rsx! { screens::eat::Eat {} },
                    Mode::Shoot => rsx! { screens::shoot::Shoot {} },
                    Mode::Hunt => rsx! { screens::hunt::Hunt {} },
                    Mode::Flee => rsx! { screens::flee::Flee {} },
                    Mode::Climb => rsx! { screens::climb::Climb {} },
                    Mode::Fog => rsx! { screens::fog::Fog {} },
                    Mode::Riders => rsx! { screens::riders::Riders {} },
                    Mode::Interaction => rsx! { screens::interaction_host::InteractionHost {} },
                    Mode::GameOver => rsx! { screens::game_over::GameOver {} },
                    }
                }
            }
        }
    }
}
