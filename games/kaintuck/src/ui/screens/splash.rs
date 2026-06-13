//! Title screen with the hall of fame.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use retro_kit::components::score_board::ScoreBoard;
use retro_kit::components::stat_row::StatRow;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN, BTN_PRIMARY, BTN_WIDE, PANEL, SCREEN_HERO};

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut on_splash = use_context::<Signal<bool>>();
    let scores = use_hook(storage::high_scores);
    let mut house = use_signal(storage::ledger);
    // The online hall of fame (top runs across all players), when this build is
    // served by a CMS. Absent/empty/errored ⇒ the panel simply doesn't render
    // and the local board below stands alone.
    let online = use_resource(|| async {
        retro_kit::leaderboard::fetch_top("kaintuck", "main", 20).await
    });
    let online_rows: Vec<(String, String)> = match &*online.read_unchecked() {
        Some(Ok(entries)) => entries
            .iter()
            .map(|e| {
                let won = e.payload.get("won").and_then(|v| v.as_bool()).unwrap_or(false);
                let rank = e.payload.get("rank").and_then(|v| v.as_str()).unwrap_or("");
                let tag = if won { "✓ home" } else { "✗" };
                (e.handle.clone(), format!("{} · {} · {}", e.score, rank, tag))
            })
            .collect(),
        _ => Vec::new(),
    };
    // A save loads in a gameplay mode; a fresh game sits on Splash.
    let resumable = game.peek().mode != Mode::Splash;

    let new_game = move |_| {
        game.write().mode = Mode::NewGame;
        on_splash.set(false);
    };
    let (start_class, start_label) = if resumable {
        (BTN_WIDE.to_string(), "NEW GAME")
    } else {
        (format!("{BTN_PRIMARY} text-xl"), "COMMISSION A FLATBOAT")
    };

    let version = env!("CARGO_PKG_VERSION");

    rsx! {
        div { class: "{SCREEN_HERO} gap-6 p-6",
            div {
                h1 { class: "splash-title", "KAINTUCK" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "Run a flatboat down the Ohio and Mississippi to Natchez, sell everything, then walk 450 miles home up the Natchez Trace. Two phases. One life. Don't get robbed."
                }
            }
            div { class: "w-full max-w-xs flex flex-col gap-3",
                if resumable {
                    button {
                        class: "{BTN_PRIMARY} text-xl",
                        onclick: move |_| on_splash.set(false),
                        "RESUME JOURNEY"
                    }
                }
                button { class: "{start_class}", onclick: new_game, "{start_label}" }
            }
            if let Some(l) = house() {
                div { class: "{PANEL} p-4 w-full max-w-xs text-left text-sm flex flex-col gap-1",
                    p { class: "text-center tracking-widest opacity-80 mb-1", "YOUR TRADING HOUSE" }
                    StatRow { label: "Fortune".to_string(), value: format!("${}", group_thousands(l.cash as i64)) }
                    StatRow { label: "Reputation".to_string(), value: (l.reputation as i64).to_string() }
                    StatRow { label: "Journeys".to_string(), value: l.journeys.to_string() }
                    StatRow { label: "Reached home".to_string(), value: l.arrivals.to_string() }
                    StatRow { label: "Best score".to_string(), value: group_thousands(l.best_score) }
                    StatRow { label: "Miles travelled".to_string(), value: group_thousands(l.lifetime_miles) }
                    button {
                        class: "{BTN} mt-2 text-xs",
                        onclick: move |_| {
                            storage::clear_ledger();
                            house.set(None);
                        },
                        "DISBAND THE HOUSE"
                    }
                }
            }
            if !online_rows.is_empty() {
                ScoreBoard {
                    title: "HALL OF FAME".to_string(),
                    rows: online_rows,
                }
            }
            ScoreBoard {
                title: "KAINTUCKS REMEMBERED".to_string(),
                rows: scores
                    .iter()
                    .map(|hs| {
                        let tag = if hs.won { "✓ home".to_string() } else { "✗".to_string() };
                        (hs.trader.clone(), format!("{} · {} · {}", hs.score, hs.rank, tag))
                    })
                    .collect::<Vec<_>>(),
            }
            p { class: "text-xs opacity-60", "After the river trade of the Kaintucks, c. 1800–1820, as told along the Natchez Trace" }
            p { class: "text-xs opacity-50", "Ported to mobile by Tony Bierman" }
            p { class: "text-xs opacity-40", "v{version}" }
        }
    }
}
