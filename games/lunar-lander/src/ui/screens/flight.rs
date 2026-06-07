//! The descent: telemetry log, altitude strip, and the burn controls. Turn
//! results reveal line-by-line into a shared `pending` signal (the status
//! chips and descent strip track the same reveal), then commit to the
//! engine log in one write once playback finishes.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::engine::state::{LogLine, MissionKind};
use crate::engine::Game;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::playback::play_paced;
use retro_kit::theme::{ACTION_BAR, BTN, PANEL};

use crate::ui::components::descent_strip::DescentStrip;

const LINE_MS: u32 = 350;
const CONTACT_MS: u32 = 1_000;

#[component]
pub fn Flight() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut pending = use_context::<Signal<Vec<LogLine>>>();
    let mut busy = use_signal(|| false);

    let mut do_burn = move |burn: i64| {
        if *busy.peek() || game.peek().decided() {
            return;
        }
        busy.set(true);
        spawn(async move {
            let lines = game.write().take_turn(burn);
            play_paced(lines, LINE_MS, false, move |line| pending.write().push(line)).await;
            let revealed: Vec<LogLine> = pending.write().drain(..).collect();
            game.write().extend_log(revealed);
            if game.peek().decided() {
                TimeoutFuture::new(CONTACT_MS).await;
                game.write().finish_flight();
            }
            busy.set(false);
        });
    };

    let g = game.read();
    let p = pending.read();
    let mission = g.mission();
    let max = g.burn_max();
    let locked = busy() || g.outcome.is_some();

    let (alt_head, presets, prompt) = match mission {
        MissionKind::Lunar => (
            "MI + FT",
            [0, 8, 50, 100, 200].as_slice(),
            "Burn rate for the next 10 seconds, lb/s. Ignition occurs at 8.",
        ),
        MissionKind::Rocket => (
            "FEET",
            [0, 5, 10, 20, 30].as_slice(),
            "Fuel units to burn this second. Each unit slows you 1 ft/s.",
        ),
    };
    let lock_class = if locked { "opacity-50 pointer-events-none" } else { "" };

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0",
            div { class: "flex-1 flex min-h-0 gap-2 px-3 pt-2",
                div { class: "{PANEL} flex-1 flex flex-col min-h-0 text-xs",
                    div { class: "log-grid chip-label border-b border-current/30 px-2 py-1",
                        span { "SEC" }
                        span { "{alt_head}" }
                        span { "SPEED" }
                        span { "FUEL" }
                        span { class: "text-right", "BURN" }
                    }
                    div { class: "flex-1 overflow-y-auto flex flex-col-reverse px-2 py-1",
                        div {
                            for (i, line) in g.log.iter().chain(p.iter()).enumerate() {
                                match line {
                                    LogLine::Row(row) => rsx! {
                                        div { key: "{i}", class: "log-grid py-0.5",
                                            span { "{row.sec}" }
                                            span { "{row.alt}" }
                                            span { "{row.vel}" }
                                            span { "{row.fuel}" }
                                            span { class: "text-right opacity-80", "{row.burn}" }
                                        }
                                    },
                                    LogLine::Banner(text) => rsx! {
                                        div { key: "{i}", class: "log-banner py-1", "{text}" }
                                    },
                                }
                            }
                        }
                    }
                }
                DescentStrip {}
            }
            div { class: "{ACTION_BAR} {lock_class}",
                div { class: "flex gap-1 px-3 pt-2",
                    for b in presets.iter().copied() {
                        button {
                            key: "{b}",
                            class: "{BTN} flex-1 text-sm",
                            onclick: move |_| do_burn(b),
                            "{b}"
                        }
                    }
                }
                NumberEntry {
                    prompt: prompt.to_string(),
                    max,
                    confirm: "BURN".to_string(),
                    on_submit: do_burn,
                }
            }
        }
    }
}
