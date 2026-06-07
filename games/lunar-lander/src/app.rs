//! App root: owns the one `Signal<Game>`, dispatches the current screen,
//! and persists every transition to localStorage.

use dioxus::prelude::*;

use crate::engine::scoring::HighScore;
use crate::engine::state::{LogLine, Mode};
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
        document::Title { "Lunar Lander" }
        GameRoot {}
    }
}

#[component]
fn GameRoot() -> Element {
    let mut game = use_signal(storage::load_or_new);
    use_context_provider(|| game);
    // Mid-playback lines being revealed; not part of the saved Game. The
    // flight screen pushes here and commits to game.log once per turn, so
    // autosave fires per turn, not per paced line.
    let pending = use_signal(Vec::<LogLine>::new);
    use_context_provider(|| pending);

    // Persist mid-flight; finished descents clear the save and, when the
    // craft survived, post a flight record exactly once.
    use_effect(move || {
        let snapshot = game.read();
        match snapshot.mode {
            Mode::Splash | Mode::MissionSelect => {}
            Mode::GameOver => {
                storage::clear_save();
                if let Some(end) = snapshot.outcome.clone().filter(|e| !e.recorded) {
                    if end.quality.landed_ok() {
                        storage::record_score(HighScore {
                            mission: end.mission,
                            score: end.score,
                            quality: end.quality.label().to_string(),
                            impact: end.impact.clone(),
                        });
                    }
                    drop(snapshot);
                    if let Some(out) = game.write().outcome.as_mut() {
                        out.recorded = true;
                    }
                }
            }
            Mode::Flight => storage::save(&snapshot),
        }
    });

    let mode = game.read().mode.clone();
    let in_game = matches!(mode, Mode::Flight);

    rsx! {
        div { class: "crt flex flex-col h-[100dvh] overflow-hidden",
            if in_game {
                StatusBar {}
            }
            div { class: "flex-1 overflow-y-auto flex flex-col",
                match mode {
                    Mode::Splash => rsx! { screens::splash::Splash {} },
                    Mode::MissionSelect => rsx! { screens::mission_select::MissionSelect {} },
                    Mode::Flight => rsx! { screens::flight::Flight {} },
                    Mode::GameOver => rsx! { screens::game_over::GameOver {} },
                }
            }
        }
    }
}
