//! One generic minigame host. The eight data-driven minigame Modes (Steady /
//! Sequence / Brigade / Splint / Dose / Fog / Flee / Climb) all route here; it
//! reads the live minigame's params from the scenario and renders the matching
//! minigames-kit component, forwarding the result to the right `resolve_*`.
//! Matching on `(Mode, MinigameParams)` in one place means the screen a hazard
//! launches is always paired with the tier its resolver applies — the scenario
//! consistency test guarantees each id's params variant matches its task kind.
//!
//! The gunfight (Shoot) and the hunting gallery (Hunt) keep their own screens:
//! their props (the situation-dependent gunfight prompt and shuffle seed, the
//! marksmanship-scaled hunt ammo) don't fit a single `MinigameParams` entry.

use dioxus::prelude::*;

use minigames_kit::bucket_brigade::{BucketBrigade, BucketBrigadeResult};
use minigames_kit::crowd_threading::{CrowdThreading, CrowdThreadingResult};
use minigames_kit::sequence::{Sequence as SequenceGame, SequenceResult};
use minigames_kit::steady_hands::{SteadyHands, SteadyHandsResult};
use minigames_kit::timing_bar::{TimingBar, TimingResult};
use trail_kit::fortnash::MinigameParams;

use crate::engine::state::Mode;
use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::theme::SCREEN_CENTERED;

#[component]
pub fn Minigame() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let mode = g.mode.clone();
    let Some(params) = g.active_mini_params() else {
        return rsx! { StuckMinigame {} };
    };
    // Reuse each original screen's per-encounter seed salt, so layouts are
    // identical to the hand-written screens (deterministic for a given save,
    // distinct between co-located encounters).
    let seed = match mode {
        Mode::Steady => g.encounter_seed(0x57EA_D70F_57EA_D70F),
        Mode::Sequence => g.encounter_seed(0x5E9E_9CE0_5E9E_9CE0),
        Mode::Brigade => g.encounter_seed(0xB0CC_E7B0_CCE7_B0CC),
        Mode::Splint => g.encounter_seed(0x5B0E_5E77_B04E_5E77),
        Mode::Dose => g.encounter_seed(0xD05E_D05E_D05E_D05E),
        Mode::Fog => g.encounter_seed(0xF06F_06F0_F06F_06F0),
        Mode::Flee => g.encounter_seed(0),
        Mode::Climb => g.encounter_seed(0x5C1F_F00D_C0DE_5A17),
        _ => return rsx! { StuckMinigame {} },
    };
    drop(g);

    match (mode, params) {
        (
            Mode::Steady,
            MinigameParams::Steady {
                prompt,
                tolerance,
                duration_ms,
                drift_speed,
            },
        ) => rsx! {
            SteadyHands {
                prompt: prompt.clone(),
                tolerance: *tolerance,
                duration_ms: *duration_ms,
                drift_speed: *drift_speed,
                seed,
                on_complete: move |res: SteadyHandsResult| {
                    game.write().resolve_steady(res.steady, res.accuracy);
                },
            }
        },
        (
            Mode::Sequence,
            MinigameParams::Sequence {
                prompt,
                symbols,
                length,
                show_ms,
            },
        ) => rsx! {
            SequenceGame {
                key: "{seed}",
                prompt: prompt.clone(),
                symbols: symbols.clone(),
                length: *length,
                show_ms: *show_ms,
                seed,
                on_complete: move |res: SequenceResult| {
                    game.write().resolve_sequence(res.correct_prefix, res.length, res.perfect);
                },
            }
        },
        (
            Mode::Brigade,
            MinigameParams::Brigade {
                prompt,
                threat_icon,
                initial_active,
                spread_ms,
                spread_chance,
                duration_ms,
            },
        ) => rsx! {
            BucketBrigade {
                key: "{seed}",
                prompt: prompt.clone(),
                threat_icon: threat_icon.clone(),
                initial_active: *initial_active,
                spread_ms: *spread_ms,
                spread_chance: *spread_chance,
                duration_ms: *duration_ms,
                seed,
                on_complete: move |res: BucketBrigadeResult| {
                    game.write().resolve_brigade(res.contained, res.leaked, res.capacity);
                },
            }
        },
        // The timing games (setting a bone, measuring a dose) share one render;
        // the live `mode` only picks which resolver the strike feeds. Each
        // resolver guards on its own mode, so a mis-routed strike is a no-op (and
        // would visibly fail to advance) rather than silently applying the wrong
        // toll.
        (
            mode,
            MinigameParams::Timing {
                prompt,
                action,
                tolerance,
                period_ms,
            },
        ) => rsx! {
            TimingBar {
                prompt: prompt.clone(),
                action: action.clone(),
                tolerance: *tolerance,
                period_ms: *period_ms,
                seed,
                on_strike: move |res: TimingResult| {
                    let mut g = game.write();
                    match mode {
                        Mode::Splint => g.resolve_splint(res.hit, res.accuracy),
                        Mode::Dose => g.resolve_dose(res.hit, res.accuracy),
                        _ => g.skip_unrenderable_minigame(),
                    }
                },
            }
        },
        // The route-memory games (the fog trace, the escape through the breaks,
        // the rugged climb) likewise share one render and differ only in which
        // resolver the result feeds.
        (
            mode,
            MinigameParams::Crowd {
                prompt,
                crowd_size,
                member_icon,
                player_icon,
                exit_icon,
            },
        ) => rsx! {
            CrowdThreading {
                prompt: prompt.clone(),
                crowd_size: *crowd_size,
                member_icon: member_icon.clone(),
                player_icon: player_icon.clone(),
                exit_icon: exit_icon.clone(),
                seed,
                on_complete: move |res: CrowdThreadingResult| {
                    let mut g = game.write();
                    match mode {
                        Mode::Fog => g.resolve_fog(res.cleared, res.accuracy),
                        Mode::Flee => g.resolve_flee(res.cleared, res.accuracy),
                        Mode::Climb => g.resolve_climb(res.cleared, res.accuracy),
                        _ => g.skip_unrenderable_minigame(),
                    }
                },
            }
        },
        // A mode/params kind mismatch (guarded against by the consistency test);
        // recover rather than strand the player on a blank screen.
        _ => rsx! { StuckMinigame {} },
    }
}

/// Shown when the live minigame can't be rendered — `active_mini_params` came
/// back `None`, or the `(Mode, params)` kinds don't line up. The consistency
/// test makes this unreachable from a well-formed scenario, but a desynced
/// save-load could still land here; rather than a blank, undismissable screen,
/// offer a button that bails out of the minigame as a no-cost clean pass.
#[component]
fn StuckMinigame() -> Element {
    let mut game = use_context::<Signal<Game>>();
    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            p { class: "text-center chip-danger p-2",
                "The trail blurs for a moment — then clears. Press on."
            }
            MenuButton {
                title: "🛞 Press on down the road ▸".to_string(),
                primary: true,
                onclick: move |_| game.write().skip_unrenderable_minigame(),
            }
        }
    }
}
