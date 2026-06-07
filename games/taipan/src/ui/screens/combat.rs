//! Sea battle: tap Fight / Run / Throw Cargo; rounds resolve in the engine
//! and the messages play back with skippable pacing.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::engine::interaction::Order;
use crate::engine::state::ITEM_NAMES;
use crate::engine::Game;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::playback::play_paced;
use crate::ui::components::ship_display::ShipDisplay;
use retro_kit::theme::{ACTION_BAR, BTN, PANEL};

const MESSAGE_MS: u32 = 550;

#[component]
pub fn Combat() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut log = use_signal(Vec::<String>::new);
    let mut busy = use_signal(|| false);
    // Some(item) while picking how much cargo to throw.
    let mut throwing = use_signal(|| Option::<usize>::None);

    let mut order_up = move |order: Order| {
        if *busy.peek() {
            return;
        }
        busy.set(true);
        spawn(async move {
            let msgs = game.write().battle_order(order);
            play_paced(msgs, MESSAGE_MS, true, move |msg| log.write().push(msg)).await;
            let decided = game
                .peek()
                .battle
                .as_ref()
                .map(|b| b.outcome.is_some())
                .unwrap_or(true);
            if decided {
                TimeoutFuture::new(MESSAGE_MS).await;
                game.write().finish_battle();
            }
            busy.set(false);
        });
    };

    let g = game.read();
    let Some(battle) = g.battle.clone() else {
        // Battle already resolved; nothing to draw while the mode flips.
        return rsx! {};
    };
    let intro = battle.intro();
    let ships = battle.ships;
    let guns = g.state.guns;
    let sw = g.state.seaworthiness();
    let has_cargo = g.state.cargo_aboard() > 0;
    let hold = g.state.hold;
    drop(g);

    // Throw-cargo picker overlays the order bar.
    if let Some(item) = throwing() {
        return rsx! {
            div { class: "flex-1 flex flex-col justify-center max-w-md w-full mx-auto",
                NumberEntry {
                    prompt: format!("How much {} shall we throw overboard, Taipan?", ITEM_NAMES[item]),
                    max: hold[item],
                    confirm: "Throw!".to_string(),
                    on_submit: move |amount| {
                        throwing.set(None);
                        order_up(Order::Throw { item, amount });
                    },
                    on_cancel: move |_| throwing.set(None),
                }
            }
        };
    }

    rsx! {
        div { class: "flex-1 flex flex-col",
            div { class: "battle-flash shrink-0 text-center py-2 px-3",
                p { class: "text-lg", "{intro}" }
                p { class: "text-sm opacity-80",
                    "{ships} remaining · {guns} guns · {sw}% seaworthy"
                }
            }
            ShipDisplay { battle }
            div { class: "{PANEL} flex-1 overflow-y-auto m-3 p-3 text-sm leading-relaxed",
                for (i, line) in log.read().iter().enumerate() {
                    p { key: "{i}", "{line}" }
                }
                if log.read().is_empty() {
                    p { class: "opacity-70", "We'll fight 'em or run, Taipan — your orders?" }
                }
            }
            div { class: "{ACTION_BAR}",
                div { class: "grid grid-cols-3 gap-2",
                    button {
                        class: "{BTN} py-3",
                        disabled: busy(),
                        onclick: move |_| order_up(Order::Fight),
                        "FIGHT"
                    }
                    button {
                        class: "{BTN} py-3",
                        disabled: busy(),
                        onclick: move |_| order_up(Order::Run),
                        "RUN"
                    }
                    button {
                        class: "{BTN} py-3",
                        disabled: busy() || !has_cargo,
                        onclick: move |_| {
                            // Default to the first commodity actually aboard.
                            if let Some(item) = (0..4).find(|&i| hold[i] > 0) {
                                throwing.set(Some(item));
                            }
                        },
                        "THROW CARGO"
                    }
                }
            }
        }
    }
}
