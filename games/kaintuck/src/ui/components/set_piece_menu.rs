//! One generic renderer for every set-piece menu (the Falls, Natchez, the
//! stands). It takes a scenario option list, hides options whose gate isn't met,
//! disables the ones you can't afford, fills the `{cost}` token, and reports the
//! chosen option's action tag back to the screen — which dispatches it (engine
//! ops go through `Game::run_set_piece`; screen navigation is handled inline).

use dioxus::prelude::*;

use trail_kit::{Gate, SetPieceOption};

use crate::engine::state::fmt_money;
use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;

/// A prepared button, evaluated against the live game state.
struct Btn {
    title: String,
    hint: String,
    primary: bool,
    disabled: bool,
    action: String,
    cost: f64,
}

#[component]
pub fn SetPieceMenu(
    options: &'static [SetPieceOption<Gate>],
    onselect: EventHandler<(String, f64)>,
) -> Element {
    let game = use_context::<Signal<Game>>();
    let g = game.read();
    let cash = g.state.cash;

    // Read the static option list by reference; only the per-visible-button
    // strings the MenuButton owns are cloned (not the whole menu).
    let buttons: Vec<Btn> = options
        .iter()
        .filter(|o| gate_ok(&g, &o.gate))
        .map(|o| {
            // Disabled when a disable-gate fails or the option is unaffordable;
            // a disable-gate (unlike the hiding `gate`) keeps the button visible
            // and swaps in its disabled hint.
            let gated_off = !gate_ok(&g, &o.disable_gate);
            let unaffordable = o.affordable_only && cash < o.cost;
            let disabled = gated_off || unaffordable;
            let hint = match (gated_off, &o.disabled_hint) {
                (true, Some(h)) => h.clone(),
                _ => o.hint.clone(),
            };
            Btn {
                title: if o.label.contains("{cost}") {
                    o.label.replace("{cost}", &fmt_money(o.cost))
                } else {
                    o.label.clone()
                },
                hint,
                primary: o.primary,
                disabled,
                action: o.action.clone(),
                cost: o.cost,
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
                onclick: move |_| onselect.call((b.action.clone(), b.cost)),
            }
        }
    }
}

/// Whether every gate on an option holds for the current game state.
fn gate_ok(g: &Game, gates: &[Gate]) -> bool {
    gates.iter().all(|gate| match gate {
        Gate::NoHorse => !g.state.has_horse,
        Gate::HasCargo => g.state.cargo_units() > 0,
        Gate::HasBoat => g.state.boat.is_some(),
        Gate::AtStand(key) => g.current_stand().is_some_and(|s| s.key() == key),
    })
}
