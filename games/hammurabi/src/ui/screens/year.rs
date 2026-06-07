//! The reign: the steward's report scroll plus the four yearly prompts.
//! Newly appended report lines reveal one by one via a shared `revealed`
//! counter, so a reload mid-playback never loses lines (they're already in
//! the saved log — a fresh mount just shows everything at once).

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::engine::state::Phase;
use crate::engine::{seed_cost, Game, BUSHELS_PER_PERSON};
use crate::ui::components::status_bar::ChipStats;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::format::group_thousands;
use retro_kit::playback::play_paced;
use retro_kit::theme::{ACTION_BAR, PANEL};

const LINE_MS: u32 = 420;
const END_MS: u32 = 1_000;

/// Pace `revealed` up to the engine log's length, one line per beat.
/// Input stays locked while `revealed < log.len()`, so the target is fixed
/// for the duration of the reveal.
async fn reveal_rest(game: Signal<Game>, mut revealed: Signal<usize>) {
    let pending: Vec<usize> = (*revealed.peek()..game.peek().log.len()).collect();
    play_paced(pending, LINE_MS, true, move |_| *revealed.write() += 1).await;
}

#[component]
pub fn Year() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let revealed = use_context::<Signal<usize>>();
    let mut frozen = use_context::<Signal<ChipStats>>();
    let mut busy = use_signal(|| false);

    // A fresh reign mounts with its year-1 report unrevealed; pace it.
    use_future(move || reveal_rest(game, revealed));

    let submit = move |n: i64| {
        if *busy.peek() || *revealed.peek() < game.peek().log.len() {
            return;
        }
        busy.set(true);
        // Freeze the chips at pre-decision values: if this submit resolves
        // the year, the status bar must not lead the paced report.
        frozen.set(ChipStats::of(&game.peek()));
        spawn(async move {
            game.write().submit(n);
            reveal_rest(game, revealed).await;
            if game.peek().decided() {
                TimeoutFuture::new(END_MS).await;
                game.write().finish_year();
            }
            busy.set(false);
        });
    };

    let g = game.read();
    let shown = (*revealed.read()).min(g.log.len());
    let locked = busy() || shown < g.log.len();
    let lock_class = if locked { "opacity-50 pointer-events-none" } else { "" };
    let price = g.state.land_price;
    let population = g.state.population;
    let phase = g.state.phase;

    let (prompt, max, confirm, preview): (String, i64, &str, Callback<i64, String>) = match phase {
        Phase::BuyLand => (
            format!("Land is trading at {price} bushels per acre. How many acres do you wish to buy?"),
            g.max_buy(),
            "BUY",
            Callback::new(move |n: i64| {
                if n == 0 {
                    "buy none — the steward will offer to sell".to_string()
                } else {
                    format!("costs {} bushels", group_thousands(n * price))
                }
            }),
        ),
        Phase::SellLand => (
            "How many acres do you wish to sell?".to_string(),
            g.max_sell(),
            "SELL",
            Callback::new(move |n: i64| format!("yields {} bushels", group_thousands(n * price))),
        ),
        Phase::Feed => (
            "How many bushels do you wish to feed your people?".to_string(),
            g.max_feed(),
            "FEED",
            Callback::new(move |n: i64| {
                format!(
                    "feeds {} of {} people (20 each)",
                    group_thousands((n / BUSHELS_PER_PERSON).min(population)),
                    group_thousands(population),
                )
            }),
        ),
        Phase::Plant => (
            "How many acres do you wish to plant with seed?".to_string(),
            g.max_plant(),
            "PLANT",
            Callback::new(move |n: i64| {
                format!("needs {} bushels of seed", group_thousands(seed_cost(n)))
            }),
        ),
    };

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0",
            div { class: "{PANEL} flex-1 flex flex-col min-h-0 mx-3 mt-2 text-sm",
                div { class: "flex-1 overflow-y-auto flex flex-col-reverse px-2 py-1",
                    div {
                        for (i, line) in g.log.iter().take(shown).enumerate() {
                            p {
                                key: "{i}",
                                class: "log-line",
                                class: if line.alert { "log-alert" },
                                class: if line.head { "log-head" },
                                "{line.text}"
                            }
                        }
                    }
                }
            }
            div { class: "{ACTION_BAR} {lock_class}",
                NumberEntry {
                    prompt,
                    max,
                    confirm: confirm.to_string(),
                    preview,
                    on_submit: submit,
                }
            }
        }
    }
}
