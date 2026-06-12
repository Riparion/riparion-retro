//! The Pittsburgh boatyard: commission a boat, hire a crew, load cargo, cast off.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::state::{BoatKind, Mode, GOOD_NAMES};
use crate::engine::Game;
use retro_kit::components::error_banner::ErrorBanner;
use retro_kit::components::seg_button::SegButton;
use retro_kit::format::fmt_dollars_compact;
use retro_kit::theme::{BTN, BTN_PRIMARY, PANEL, SCREEN};

#[component]
pub fn Pittsburgh() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let built = game.read().has_boat();

    if built {
        return rsx! { LoadAndDepart {} };
    }

    let mut kind = use_signal(|| BoatKind::Flatboat);
    let mut crew = use_signal(|| 3i64);
    let mut error = use_signal(String::new);

    let g = game.read();
    let funds = g.state.cash + (g.state.credit_cap - g.state.debt).max(0.0);
    drop(g);

    let cost = kind().cost() + crew() as f64 * scenario().start.crew_wage;

    let build = move |_| {
        match game.write().build(kind(), crew()) {
            Ok(()) => {}
            Err(e) => error.set(e),
        }
    };

    rsx! {
        div { class: "{SCREEN} gap-3",
            div { class: "text-center",
                h2 { class: "text-lg tracking-widest", "COMMISSION A BOAT" }
                p { class: "opacity-80 text-sm", "You have {fmt_dollars_compact(funds)} to raise (cash and boatyard credit)." }
            }
            div { class: "{PANEL} p-3 flex flex-col gap-2",
                div { class: "chip-label", "── THE BOAT ──" }
                for b in BoatKind::ALL {
                    SegButton {
                        label: format!("{} — {} units, ${}", b.label(), b.capacity(), b.cost() as i64),
                        active: kind() == b,
                        onclick: move |_| { kind.set(b); error.set(String::new()); },
                    }
                }
                p { class: "text-xs opacity-60", "{kind().hint()}" }
            }
            div { class: "{PANEL} p-3 flex flex-col gap-2",
                div { class: "chip-label", "── CREW ($3 a hand) ──" }
                div { class: "flex gap-2",
                    for n in 1..=5i64 {
                        SegButton {
                            label: n.to_string(),
                            active: crew() == n,
                            onclick: move |_| { crew.set(n); error.set(String::new()); },
                        }
                    }
                }
                p { class: "text-xs opacity-60", "More hands cost more but weather the river better." }
            }
            div { class: "text-center text-lg", "Total: {fmt_dollars_compact(cost)}" }
            ErrorBanner { message: (!error.read().is_empty()).then(|| error.read().clone()) }
            button { class: "{BTN_PRIMARY} py-4 text-lg", onclick: build, "BUILD HER ▸" }
        }
    }
}

#[component]
fn LoadAndDepart() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let rows: Vec<(usize, i64)> = (0..GOOD_NAMES.len()).map(|i| (i, s.hold[i])).collect();
    let cargo = s.cargo_units();
    let cap = s.capacity();
    drop(g);

    rsx! {
        div { class: "{SCREEN} gap-3",
            div { class: "text-center",
                h2 { class: "text-lg tracking-widest", "LOAD THE BOAT" }
                p { class: "opacity-80 text-sm", "Hold: {cargo} of {cap} units full." }
            }
            div { class: "{PANEL} p-3",
                div { class: "chip-label mb-1", "── ABOARD ──" }
                for (i, n) in rows {
                    if n > 0 {
                        div { key: "{i}", class: "flex justify-between text-sm",
                            span { "{GOOD_NAMES[i]}" }
                            span { "{n}" }
                        }
                    }
                }
                if cargo == 0 {
                    p { class: "text-sm opacity-60", "Empty. Buy cheap cargo here to sell downriver." }
                }
            }
            button {
                class: "{BTN}",
                onclick: move |_| game.write().mode = Mode::Trade { buying: true },
                "BUY CARGO"
            }
            button {
                class: "{BTN_PRIMARY} py-4 text-lg",
                onclick: move |_| game.write().depart(),
                "CAST OFF ⛵ ▸"
            }
        }
    }
}
