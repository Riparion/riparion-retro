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
        document::Title { "Taipan!" }
        GameRoot {}
    }
}

#[component]
fn GameRoot() -> Element {
    let mut game = use_signal(storage::load_or_new);
    use_context_provider(|| game);

    // Persist every transition; finished games clear the save and post a score.
    use_effect(move || {
        let snapshot = game.read();
        match snapshot.mode {
            Mode::Splash | Mode::NewGame => {}
            Mode::GameOver => {
                storage::clear_save();
                if let Some(end) = snapshot.outcome.clone().filter(|e| !e.recorded) {
                    storage::record_score(HighScore {
                        firm: snapshot.state.firm.clone(),
                        score: end.score,
                        rank: end.rank.clone(),
                        years: end.years,
                        months: end.months,
                        retired: end.retired,
                    });
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
    let in_game = !matches!(mode, Mode::Splash | Mode::NewGame | Mode::GameOver);

    rsx! {
        div { class: "crt flex flex-col h-[100dvh] overflow-hidden",
            if in_game {
                StatusBar {}
            }
            div { class: "flex-1 overflow-y-auto flex flex-col",
                match mode {
                    Mode::Splash => rsx! { screens::splash::Splash {} },
                    Mode::NewGame => rsx! { screens::new_game::NewGame {} },
                    Mode::Port => rsx! { screens::port::Port {} },
                    Mode::Trade { buying } => rsx! { screens::trade::Trade { buying } },
                    Mode::Transfer => rsx! { screens::transfer::Transfer {} },
                    Mode::Bank => rsx! { screens::bank::Bank {} },
                    Mode::Wu => rsx! { screens::wu::Wu {} },
                    Mode::Travel => rsx! { screens::travel::Travel {} },
                    Mode::Combat => rsx! { screens::combat::Combat {} },
                    Mode::Interaction => rsx! { screens::interaction_host::InteractionHost {} },
                    Mode::GameOver => rsx! { screens::game_over::GameOver {} },
                }
            }
        }
    }
}
