//! App root: owns the one `Signal<Game>`, dispatches the current screen, and
//! persists every transition to localStorage.

use dioxus::prelude::*;
use retro_kit::game_flow::{use_persistence, Persisted};

use crate::engine::scoring::HighScore;
use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use crate::ui::components::cover::Cover;
use crate::ui::components::status_bar::StatusBar;
use crate::ui::screens;

impl Persisted for Game {
    fn is_transient(&self) -> bool {
        matches!(self.mode, Mode::Splash | Mode::NewGame)
    }
    fn is_game_over(&self) -> bool {
        matches!(self.mode, Mode::GameOver)
    }
    fn save(&self) {
        storage::save(self);
    }
    fn finish(mut game: Signal<Self>) {
        storage::clear_save();
        let snapshot = game.read();
        if let Some(end) = snapshot.outcome.clone().filter(|e| !e.recorded) {
            storage::record_score(HighScore::from_end(&snapshot.state.trader, &end));
            drop(snapshot);
            if let Some(out) = game.write().outcome.as_mut() {
                out.recorded = true;
            }
        }
    }
}

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
        document::Title { "Kaintuck" }
        GameRoot {}
    }
}

#[component]
fn GameRoot() -> Element {
    let game = use_signal(storage::load_or_new);
    use_context_provider(|| game);
    // Every load opens on the title screen, even when a save resumes a journey
    // (the splash then offers RESUME / NEW GAME). See NOTES.md "Start screen".
    let on_splash = use_signal(|| true);
    use_context_provider(|| on_splash);

    // Persist every transition; finished journeys clear the save and post a score.
    use_persistence(game);

    let mode = game.read().mode.clone();
    let show_splash = on_splash();
    let in_game = !show_splash && !matches!(mode, Mode::Splash | Mode::NewGame | Mode::GameOver);

    rsx! {
        div { class: "crt flex flex-col h-[100dvh] overflow-hidden",
            if in_game {
                StatusBar {}
            }
            div { class: "flex-1 overflow-y-auto flex flex-col",
                if show_splash {
                    screens::splash::Splash {}
                } else {
                    Cover {}
                    {match mode {
                        Mode::Splash => rsx! { screens::splash::Splash {} },
                        Mode::NewGame => rsx! { screens::new_game::NewGame {} },
                        Mode::Pittsburgh => rsx! { screens::pittsburgh::Pittsburgh {} },
                        Mode::Town => rsx! { screens::town::Town {} },
                        Mode::Trade { buying } => rsx! { screens::trade::Trade { buying } },
                        Mode::Moneylender => rsx! { screens::moneylender::Moneylender {} },
                        Mode::Falls => rsx! { screens::falls::Falls {} },
                        Mode::Natchez => rsx! { screens::natchez::Natchez {} },
                        Mode::TraceHub => rsx! { screens::trace_hub::TraceHub {} },
                        Mode::Stand => rsx! { screens::stand::Stand {} },
                        Mode::Steady => rsx! { screens::steady::Steady {} },
                        Mode::Quick => rsx! { screens::quickdraw::Quick {} },
                        Mode::Crowd => rsx! { screens::crowd::Crowd {} },
                        Mode::Timing => rsx! { screens::timing::Timing {} },
                        Mode::Sequence => rsx! { screens::sequence::Sequence {} },
                        Mode::Brigade => rsx! { screens::brigade::Brigade {} },
                        Mode::Interaction => rsx! { screens::interaction_host::InteractionHost {} },
                        Mode::GameOver => rsx! { screens::game_over::GameOver {} },
                    }}
                }
            }
        }
    }
}
