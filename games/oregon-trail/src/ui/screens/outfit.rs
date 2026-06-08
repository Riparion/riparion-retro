//! Spend the $700 grubstake across the five categories before rolling out.

use dioxus::prelude::*;

use crate::engine::state::{BULLETS_PER_DOLLAR, STARTING_CASH};
use crate::engine::Game;
use retro_kit::components::spend_row::SpendRow;
use retro_kit::theme::{BTN_PRIMARY, SCREEN};

/// (label, hint, starting suggestion)
const ROWS: [(&str, &str, f64); 5] = [
    ("Oxen team", "$200–$300; a stronger team travels faster", 240.0),
    ("Food", "you'll eat ~13–23 lbs a fortnight", 300.0),
    ("Ammunition", "$1 buys a belt of 50 bullets", 60.0),
    ("Clothing", "warmth for the mountain blizzards", 60.0),
    ("Misc. supplies", "spare parts and medicine", 40.0),
];

#[component]
pub fn Outfit() -> Element {
    let mut game = use_context::<Signal<Game>>();
    // Dollar amounts per category, as edit strings.
    let mut amounts = use_signal(|| ROWS.map(|r| r.2 as i64).to_vec());
    let mut error = use_signal(String::new);

    let vals = amounts.read().clone();
    let oxen = vals[0] as f64;
    let total: i64 = vals.iter().sum();
    let remaining = STARTING_CASH as i64 - total;
    let oxen_ok = (200..=300).contains(&vals[0]);
    let ready = oxen_ok && remaining >= 0;

    let hit_trail = move |_| {
        let v = amounts.read().clone();
        match game.write().outfit(
            v[0] as f64,
            v[1] as f64,
            v[2] as f64,
            v[3] as f64,
            v[4] as f64,
        ) {
            Ok(()) => {}
            Err(e) => error.set(e),
        }
    };

    rsx! {
        div { class: "{SCREEN} gap-3",
            div { class: "text-center",
                h2 { class: "text-lg tracking-widest", "OUTFIT THE WAGON" }
                p { class: "opacity-80 text-sm", "You have $700 after buying your wagon." }
            }
            div { class: "flex flex-col gap-2",
                for (i, (label, hint, _)) in ROWS.iter().enumerate() {
                    SpendRow {
                        key: "{i}",
                        label: label.to_string(),
                        hint: hint.to_string(),
                        note: (i == 2)
                            .then(|| format!("= {} bullets", (vals[2] as f64 * BULLETS_PER_DOLLAR) as i64)),
                        value: vals[i],
                        on_input: move |n| {
                            amounts.write()[i] = n;
                            error.set(String::new());
                        },
                    }
                }
            }
            div { class: "flex justify-between text-lg px-1",
                span { class: if oxen_ok { "" } else { "chip-danger" },
                    if !oxen_ok { "Oxen must be $200–$300" } else { "Oxen ${oxen as i64}" }
                }
                span { class: if remaining < 0 { "chip-danger" } else { "" },
                    "Left: ${remaining}"
                }
            }
            if !error.read().is_empty() {
                p { class: "text-center chip-danger p-1", "{error}" }
            }
            button {
                class: "{BTN_PRIMARY} py-4 text-lg",
                disabled: !ready,
                onclick: hit_trail,
                "HIT THE TRAIL ▸"
            }
        }
    }
}
