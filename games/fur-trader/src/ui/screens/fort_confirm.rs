//! The chosen route's full description and the original's change-your-mind
//! prompt: "DO YOU WANT TO TRADE AT ANOTHER FORT?" (lines 1120–1156).
//! SET OUT commits the trip; the results screen paces the journal.

use dioxus::prelude::*;

use crate::engine::state::fort_info;
use crate::engine::{fmt_dollars, Game};
use crate::ui::components::status_bar::ChipStats;
use retro_kit::components::stat_row::StatRow;
use retro_kit::theme::{ACTION_BAR, BTN, BTN_PRIMARY, PANEL};

#[component]
pub fn FortConfirm() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut frozen = use_context::<Signal<ChipStats>>();
    let g = game.read();
    let Some(fort) = g.state.fort else {
        return rsx! {};
    };
    let info = fort_info(fort);
    let savings = g.state.savings;
    let after = savings - info.cost();
    drop(g);

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0",
            div { class: "flex-1 overflow-y-auto flex flex-col gap-2 p-3",
                div { class: "{PANEL} p-3",
                    div { class: "flex justify-between items-baseline",
                        span { class: "font-bold", "{info.name}" }
                        span { class: "text-sm opacity-80", "{info.city}" }
                    }
                    p { class: "text-sm leading-snug mt-2 opacity-90", "{info.flavor}" }
                }
                div { class: "{PANEL} p-3 text-sm flex flex-col gap-1",
                    StatRow { label: "Supplies", value: fmt_dollars(info.supplies) }
                    StatRow { label: "Travel expenses", value: fmt_dollars(info.travel) }
                    StatRow { label: "Savings after outfitting", value: fmt_dollars(after) }
                }
                if after < 0.0 {
                    p { class: "chip-danger text-sm px-1",
                        "This trip puts the company in debt — only the sale can save you."
                    }
                }
            }
            div { class: "{ACTION_BAR} flex flex-col gap-2 p-3",
                p { class: "text-sm text-center opacity-80", "Do you want to trade at another fort?" }
                div { class: "flex gap-2",
                    button {
                        class: "{BTN} flex-1 py-3",
                        onclick: move |_| game.write().change_fort(),
                        "ANOTHER FORT"
                    }
                    button {
                        class: "{BTN_PRIMARY} flex-1 py-3",
                        onclick: move |_| {
                            // Freeze the chips at pre-trip values: the journal
                            // must not be spoiled by the header.
                            frozen.set(ChipStats::of(&game.peek()));
                            game.write().confirm_fort();
                        },
                        "SET OUT"
                    }
                }
            }
        }
    }
}
