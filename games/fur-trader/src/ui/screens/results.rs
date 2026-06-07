//! The expedition journal: travel, events and the sale reveal line by line
//! via a shared `revealed` counter, so a reload mid-playback never loses
//! lines (they're already in the saved log — a fresh mount just shows
//! everything at once). When the reveal catches up, the sale ledger and the
//! original's "DO YOU WANT TO TRADE FURS NEXT YEAR?" prompt appear.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::engine::state::FUR_ORDER;
use crate::engine::{fmt_dollars, Game};
use crate::ui::components::status_bar::ChipStats;
use retro_kit::components::stat_row::StatRow;
use retro_kit::playback::play_paced;
use retro_kit::theme::{ACTION_BAR, BTN, BTN_PRIMARY, PANEL};

const LINE_MS: u32 = 480;
const END_MS: u32 = 1_000;

/// Pace `revealed` up to the engine log's length, one line per beat.
/// The actions stay locked while `revealed < log.len()`, so the target is
/// fixed for the duration of the reveal.
async fn reveal_rest(game: Signal<Game>, mut revealed: Signal<usize>) {
    let pending: Vec<usize> = (*revealed.peek()..game.peek().log.len()).collect();
    play_paced(pending, LINE_MS, true, move |_| *revealed.write() += 1).await;
}

/// Pace whatever the last engine call appended, then flip to the reckoning
/// if it decided the run (the ambush, bankruptcy).
async fn reveal_and_finish(mut game: Signal<Game>, revealed: Signal<usize>) {
    reveal_rest(game, revealed).await;
    if game.peek().decided() {
        TimeoutFuture::new(END_MS).await;
        game.write().finish_visit();
    }
}

#[component]
pub fn Results() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let revealed = use_context::<Signal<usize>>();
    let mut frozen = use_context::<Signal<ChipStats>>();
    let mut busy = use_signal(|| false);

    // The visit was committed just before this screen mounted; pace its
    // journal entry (a reload mounts fully revealed — no-op).
    use_future(move || reveal_and_finish(game, revealed));

    let g = game.read();
    let shown = (*revealed.read()).min(g.log.len());
    let locked = busy() || shown < g.log.len() || g.decided();
    let lock_class = if locked { "opacity-50 pointer-events-none" } else { "" };
    let sale = g.state.last_sale;
    let prices = g.state.sale_prices;
    let furs = g.state.furs;

    let next_year = move |_| {
        if *busy.peek() || *revealed.peek() < game.peek().log.len() {
            return;
        }
        busy.set(true);
        frozen.set(ChipStats::of(&game.peek()));
        spawn(async move {
            game.write().next_year();
            // Bankruptcy stays on this screen and paces the bad news;
            // otherwise the mode already flipped to Allocate and this
            // task ends with the unmount.
            reveal_and_finish(game, revealed).await;
            busy.set(false);
        });
    };

    let retire = move |_| {
        if *busy.peek() || *revealed.peek() < game.peek().log.len() {
            return;
        }
        let mut g = game.write();
        g.retire();
        g.finish_visit();
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
            if !locked {
                if let Some(sale) = sale {
                    div { class: "{PANEL} mx-3 mt-2 p-3 text-sm flex flex-col gap-1",
                        for fur in FUR_ORDER {
                            StatRow {
                                label: format!(
                                    "{} {} @ {}",
                                    furs[fur as usize],
                                    fur.name(),
                                    fmt_dollars(prices[fur as usize]),
                                ),
                                value: fmt_dollars(sale[fur as usize]),
                            }
                        }
                        StatRow {
                            label: "SALE TOTAL".to_string(),
                            value: fmt_dollars(sale.iter().sum()),
                        }
                    }
                }
            }
            div { class: "{ACTION_BAR} flex flex-col gap-2 p-3 {lock_class}",
                p { class: "text-sm text-center opacity-80", "Do you want to trade furs next year?" }
                div { class: "flex gap-2",
                    button { class: "{BTN} flex-1 py-3", onclick: retire, "RETIRE" }
                    button { class: "{BTN_PRIMARY} flex-1 py-3", onclick: next_year, "NEXT YEAR" }
                }
            }
        }
    }
}
