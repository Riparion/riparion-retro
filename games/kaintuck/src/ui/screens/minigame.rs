//! One generic minigame host. The six minigame Modes (Steady/Quick/Crowd/
//! Timing/Sequence/Brigade) all route here; it reads the live minigame's params
//! from the scenario and renders the matching minigames-kit component, forwarding
//! the result to the right `resolve_*`. Matching on the `MiniParams` variant in
//! one place means a kind/variant mismatch can't silently render six different
//! empty screens — there is one fallback, and the scenario consistency test
//! guarantees each id's params variant matches its task kind.

use dioxus::prelude::*;

use minigames_kit::bucket_brigade::{BucketBrigade, BucketBrigadeResult};
use minigames_kit::crowd_threading::{CrowdThreading, CrowdThreadingResult};
use minigames_kit::heave::{Heave, HeaveResult};
use minigames_kit::hot_cold::{HotCold, HotColdResult};
use minigames_kit::hunter::{Hunter, HunterResult};
use minigames_kit::quickdraw::{QuickDraw, QuickDrawResult};
use minigames_kit::sequence::{Sequence as SequenceGame, SequenceResult};
use minigames_kit::steady_hands::{SteadyHands, SteadyHandsResult};
use minigames_kit::timing_bar::{TimingBar, TimingResult};
use trail_kit::MiniParams;

use crate::engine::Game;

#[component]
pub fn Minigame() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let Some(params) = g.active_mini_params() else {
        return rsx! {};
    };
    // Per-kind seed salt, so co-located encounters stay distinct.
    let seed = match params {
        MiniParams::Steady { .. } => g.encounter_seed(0x57EA_D70F),
        MiniParams::Quick { .. } => g.encounter_seed(0x0BAD_F00D),
        MiniParams::Timing { .. } => g.encounter_seed(0x7131_46A1),
        MiniParams::Crowd { .. } => g.encounter_seed(0x51DE_7411),
        MiniParams::Sequence { .. } => g.encounter_seed(0x05E9_E0CE),
        MiniParams::Brigade { .. } => g.encounter_seed(0x6A11_0000),
        MiniParams::Heave { .. } => g.encounter_seed(0x4EA7_E000),
        MiniParams::HotCold { .. } => g.encounter_seed(0x40C0_1D00),
        MiniParams::Hunter { .. } => g.encounter_seed(0x40A7_E211),
    };
    // Hiding the money pays off in company: a kept night-watch (a party on the
    // road — RESEARCH_PIRATES §7) buys you time to reach your stash before
    // Mason's men do.
    let in_company = g.state.grouped || g.state.river_convoy;
    drop(g);

    match params {
        MiniParams::Steady {
            prompt,
            tolerance,
            duration_ms,
            drift_speed,
        } => rsx! {
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
        MiniParams::Quick { prompt, words } => {
            let target = (seed % words.len() as u64) as usize;
            rsx! {
                QuickDraw {
                    prompt: prompt.clone(),
                    words: words.clone(),
                    target,
                    seed,
                    on_fire: move |res: QuickDrawResult| {
                        game.write().resolve_quick(res.seconds, res.hit);
                    },
                }
            }
        }
        MiniParams::Timing {
            prompt,
            action,
            tolerance,
            period_ms,
        } => rsx! {
            TimingBar {
                prompt: prompt.clone(),
                action: action.clone(),
                tolerance: *tolerance,
                period_ms: *period_ms,
                seed,
                on_strike: move |res: TimingResult| {
                    game.write().resolve_timing(res.hit, res.accuracy);
                },
            }
        },
        MiniParams::Crowd {
            prompt,
            crowd_size,
            member_icon,
            player_icon,
            exit_icon,
            reveal_ms,
            navigate_ms,
        } => rsx! {
            CrowdThreading {
                prompt: prompt.clone(),
                crowd_size: *crowd_size,
                member_icon: member_icon.clone(),
                player_icon: player_icon.clone(),
                exit_icon: exit_icon.clone(),
                reveal_ms: *reveal_ms,
                navigate_ms: *navigate_ms,
                seed,
                on_complete: move |res: CrowdThreadingResult| {
                    game.write().resolve_crowd(res.cleared, res.accuracy);
                },
            }
        },
        MiniParams::Sequence {
            prompt,
            symbols,
            length,
        } => rsx! {
            SequenceGame {
                prompt: prompt.clone(),
                symbols: symbols.clone(),
                length: *length,
                seed,
                on_complete: move |res: SequenceResult| {
                    game.write().resolve_sequence(res.correct_prefix, res.length, res.perfect);
                },
            }
        },
        MiniParams::Brigade {
            prompt,
            threat_icon,
            cols,
            rows,
            initial_active,
            spread_ms,
            duration_ms,
        } => rsx! {
            BucketBrigade {
                prompt: prompt.clone(),
                threat_icon: threat_icon.clone(),
                cols: *cols,
                rows: *rows,
                initial_active: *initial_active,
                spread_ms: *spread_ms,
                duration_ms: *duration_ms,
                seed,
                on_complete: move |res: BucketBrigadeResult| {
                    game.write().resolve_brigade(res.contained, res.leaked, res.capacity);
                },
            }
        },
        MiniParams::Heave {
            prompt,
            stages,
            heave_rate,
            hold_ticks,
            slip_load,
        } => rsx! {
            Heave {
                prompt: prompt.clone(),
                stages: *stages,
                heave_rate: *heave_rate,
                hold_ticks: *hold_ticks,
                slip_load: *slip_load,
                seed,
                on_complete: move |res: HeaveResult| {
                    game.write().resolve_heave(res.opened, res.slips, res.grip_left);
                },
            }
        },
        MiniParams::HotCold {
            prompt,
            cols,
            rows,
            max_probes,
        } => {
            // A kept watch in company shortens the bandit's search of your camp,
            // but never the swamp (there you are the one searching for firm ground).
            let is_search = matches!(
                game.read().hotcold_task(),
                Some(crate::engine::tasks::HotColdTask::MasonSearch)
            );
            let probes = if is_search && in_company {
                max_probes + 2
            } else {
                *max_probes
            };
            rsx! {
                HotCold {
                    prompt: prompt.clone(),
                    cols: *cols,
                    rows: *rows,
                    max_probes: probes,
                    seed,
                    on_complete: move |res: HotColdResult| {
                        // Grade against the budget the encounter actually allows
                        // (`probes`), not the kit's lenient full-grid `res.par`.
                        game.write().resolve_hotcold(res.found, res.probes_used, probes);
                    },
                }
            }
        }
        MiniParams::Hunter {
            prompt,
            hunted_icon,
            cols,
            rows,
            ammo,
            step_ms,
            reload_ms,
        } => rsx! {
            Hunter {
                prompt: prompt.clone(),
                hunted_icon: hunted_icon.clone(),
                cols: *cols,
                rows: *rows,
                ammo: *ammo,
                step_ms: *step_ms,
                reload_ms: *reload_ms,
                seed,
                on_complete: move |res: HunterResult| {
                    game.write().resolve_hunter(res.hit, res.shots_fired);
                },
            }
        },
    }
}
