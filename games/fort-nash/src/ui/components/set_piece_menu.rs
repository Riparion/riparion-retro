//! One generic renderer for Fort Nash's button menus (the weekly Trail hub and
//! the war-party tactics). It takes a scenario option list, hides options whose
//! gate isn't met, and reports the chosen option's action tag back to the screen,
//! which dispatches it to the right engine op. Keeping the menu *structure* in the
//! scenario lets both menus render through one component, and a reorder or relabel
//! need no code.

use dioxus::prelude::*;

use trail_kit::fortnash::{Gate, SetPieceOption};

use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;

/// A prepared button, evaluated against the live game state.
struct Btn {
    title: String,
    hint: String,
    primary: bool,
    disabled: bool,
    action: String,
}

#[component]
pub fn SetPieceMenu(
    options: &'static [SetPieceOption<Gate>],
    onselect: EventHandler<String>,
) -> Element {
    let game = use_context::<Signal<Game>>();
    let g = game.read();

    // Read the static option list by reference; only the per-visible-button
    // strings the MenuButton owns are cloned (not the whole menu).
    let buttons: Vec<Btn> = options
        .iter()
        .filter(|o| gate_ok(&g, &o.gate))
        .map(|o| {
            // A failing disable-gate greys the button (and swaps in its
            // disabled hint) instead of hiding it — the player keeps the
            // affordance and learns why it's unavailable.
            let disabled = !gate_ok(&g, &o.disable_gate);
            let hint = match (disabled, &o.disabled_hint) {
                (true, Some(h)) => h.clone(),
                _ => o.hint.clone(),
            };
            Btn {
                title: o.label.clone(),
                hint,
                primary: o.primary,
                disabled,
                action: o.action.clone(),
            }
        })
        .collect();
    drop(g);

    rsx! {
        for b in buttons.into_iter() {
            MenuButton {
                key: "{b.action}",
                title: b.title,
                hint: b.hint,
                primary: b.primary,
                disabled: b.disabled,
                onclick: move |_| onselect.call(b.action.clone()),
            }
        }
    }
}

/// Whether every gate on an option holds for the current game state.
fn gate_ok(g: &Game, gates: &[Gate]) -> bool {
    gates.iter().all(|gate| match gate {
        Gate::FortAvailable => g.state.fort_available(),
        Gate::CanHunt => g.state.bullets > 39.0,
    })
}
