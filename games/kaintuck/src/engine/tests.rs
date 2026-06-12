//! Engine tests: per-formula units plus a multi-seed scripted full playthrough
//! (River → Natchez → Trace → an ending) that asserts the invariants hold.

use super::interaction::{Interaction, Response};
use super::state::{BoatKind, Mode, Phase};
use super::*;

fn started(seed: u64) -> Game {
    let mut g = Game::new(seed);
    g.begin("Tester".into());
    g
}

/// Resolve any minigame/interaction at the head until we sit at a hub or end.
fn settle(g: &mut Game) {
    let mut guard = 0;
    loop {
        match g.mode {
            Mode::Interaction => {
                let resp = match g.pending.front() {
                    Some(Interaction::Message { .. }) => Response::Acknowledge,
                    Some(Interaction::CrewLeaves { .. }) => Response::Yes,
                    Some(Interaction::FerryToll { .. }) => Response::Yes,
                    None => Response::Acknowledge,
                };
                g.resolve(resp);
            }
            Mode::Steady => g.resolve_steady(true, 0.9),
            Mode::Quick => g.resolve_quick(0.5, true),
            Mode::Crowd => g.resolve_crowd(true, 0.9),
            Mode::Timing => g.resolve_timing(true, 0.9),
            Mode::Sequence => g.resolve_sequence(4, 4, true),
            Mode::Brigade => g.resolve_brigade(true, 0, 9),
            _ => return,
        }
        guard += 1;
        assert!(guard < 2000, "settle never converged (mode {:?})", g.mode);
    }
}

fn check_invariants(g: &Game) {
    let s = &g.state;
    assert!(s.cash.is_finite() && s.cash >= 0.0, "cash {}", s.cash);
    assert!(s.debt >= 0.0, "debt {}", s.debt);
    assert!(s.hold.iter().all(|&n| n >= 0), "hold {:?}", s.hold);
    assert!((0.0..=100.0).contains(&s.morale), "morale {}", s.morale);
    assert!(s.health <= 100.0, "health {}", s.health);
    assert!(
        s.health > 0.0 || g.outcome.is_some(),
        "dead but no outcome (health {})",
        s.health
    );
    assert!(g.outcome.is_none() || g.mode == Mode::GameOver);
}

#[test]
fn build_costs_cash_then_credit() {
    let mut g = started(1);
    assert_eq!(g.mode, Mode::Pittsburgh);
    g.build(BoatKind::Flatboat, 3).unwrap();
    // $75 boat + 3*$3 crew = $84; start cash $50 → $34 on credit (debt).
    assert!(g.has_boat());
    assert_eq!(g.state.cash, 0.0);
    assert_eq!(g.state.debt, 34.0);
    assert_eq!(g.state.crew, 3);
    assert_eq!(g.state.capacity(), 60);
}

#[test]
fn build_rejects_the_unaffordable() {
    let mut g = started(2);
    // Broadhorn $130 + 5 crew $15 = $145 > $50 cash + $200 credit = $250? affordable.
    assert!(g.build(BoatKind::Broadhorn, 5).is_ok());
    let mut g = started(2);
    g.state.credit_cap = 10.0; // almost no credit
    assert!(g.build(BoatKind::Broadhorn, 5).is_err());
    assert!(!g.has_boat());
}

#[test]
fn buy_clamps_to_cash_and_hold() {
    let mut g = started(3);
    g.build(BoatKind::Skiff, 1).unwrap(); // 30 units, leaves little cash
    let corn = 0;
    let max = g.max_buy(corn);
    assert_eq!(max, g.max_buy(corn));
    g.buy(corn, max + 1000);
    assert_eq!(g.state.hold[corn], max);
    assert!(g.state.free_hold() >= 0);
    assert!(g.state.cash >= 0.0);
}

#[test]
fn borrow_and_repay_respect_caps() {
    let mut g = started(4);
    let cap = g.max_borrow();
    g.borrow(1e9);
    assert!((g.state.debt - cap).abs() < 1e-9);
    let start_cash = super::scenario_data::scenario().start.cash;
    assert!((g.state.cash - (start_cash + cap)).abs() < 1e-9);
    g.repay(1e9);
    assert!(g.state.debt >= 0.0 && g.state.cash >= 0.0);
}

#[test]
fn entering_the_trace_resets_for_the_walk() {
    let mut g = started(5);
    g.build(BoatKind::Flatboat, 2).unwrap();
    g.state.town = super::state::NATCHEZ;
    g.mode = Mode::Natchez;
    g.set_out_on_trace();
    assert_eq!(g.phase, Phase::Trace);
    assert_eq!(g.mode, Mode::TraceHub);
    assert_eq!(g.state.miles, 0.0);
    assert_eq!(g.state.day, 0);
    assert_eq!(g.state.health, 100.0);
    assert!(g.state.boat.is_none(), "boat broken up for lumber");
    assert_eq!(g.state.crew_at_natchez, 2);
}

#[test]
fn scripted_full_playthrough_smoke() {
    for seed in [11u64, 22, 33, 44, 55, 66, 77] {
        let mut g = started(seed);
        g.build(BoatKind::Flatboat, 3).unwrap();
        // Load up on the cheap Pittsburgh whiskey and corn.
        g.buy(1, g.max_buy(1) / 2);
        g.buy(0, g.max_buy(0));
        check_invariants(&g);

        // Run the river down to Natchez.
        let mut guard = 0;
        loop {
            settle(&mut g);
            check_invariants(&g);
            match g.mode {
                Mode::Pittsburgh | Mode::Town => g.depart(),
                Mode::Falls => g.falls_pilot(8.0),
                Mode::Natchez | Mode::GameOver => break,
                other => panic!("seed {seed}: unexpected river mode {other:?}"),
            }
            guard += 1;
            assert!(guard < 200, "seed {seed}: river never reached Natchez");
        }

        if g.mode == Mode::Natchez {
            assert_eq!(g.phase, Phase::River);
            g.set_out_on_trace();
            assert_eq!(g.phase, Phase::Trace);

            // Walk the Trace until we win or die.
            let mut guard = 0;
            loop {
                settle(&mut g);
                check_invariants(&g);
                match g.mode {
                    Mode::TraceHub => g.travel_day(),
                    Mode::Stand => {
                        g.rest_and_resupply(8.0);
                        g.leave_stand();
                    }
                    Mode::GameOver => break,
                    other => panic!("seed {seed}: unexpected trace mode {other:?}"),
                }
                guard += 1;
                assert!(guard < 200, "seed {seed}: trace never ended (mode {:?})", g.mode);
            }
        }

        assert!(g.outcome.is_some(), "seed {seed}: game never resolved");
        let end = g.outcome.unwrap();
        assert!(!end.rank.is_empty());
    }
}

#[test]
fn pushing_a_hard_pace_with_no_food_eventually_kills() {
    let mut g = started(9);
    g.build(BoatKind::Skiff, 1).unwrap();
    g.state.town = super::state::NATCHEZ;
    g.mode = Mode::Natchez;
    g.set_out_on_trace();
    g.state.provisions = 0.0;
    // Hard pace, starving — health bleeds out within the day cap.
    let mut guard = 0;
    while g.outcome.is_none() {
        if g.mode == Mode::TraceHub {
            g.set_pace(super::state::Pace::Hard);
            g.travel_day();
        } else {
            settle(&mut g);
            if g.mode == Mode::Stand {
                g.leave_stand();
            }
        }
        guard += 1;
        assert!(guard < 300);
    }
    assert!(g.outcome.is_some());
}

/// Parks the walker at the Tennessee Divide stand with the day's travel having
/// already vaulted past the Duck River (440) and the 450-mile finish.
fn at_divide_having_vaulted_the_river(seed: u64, has_horse: bool) -> Game {
    let mut g = started(seed);
    g.state.town = super::state::NATCHEZ;
    g.mode = Mode::Natchez;
    g.set_out_on_trace();
    g.state.has_horse = has_horse;
    g.state.stand_idx = 4; // MountLocust..TnDivide consumed; DuckRiver is next
    g.state.miles = 452.0;
    g.mode = Mode::Stand;
    g.leg = None;
    g.resume = Resume::NextDay;
    g
}

#[test]
fn a_fast_final_day_still_crosses_the_duck_river_on_horseback() {
    let mut g = at_divide_having_vaulted_the_river(7, true);
    g.leave_stand();
    settle(&mut g); // flush the crossing narration
    assert_eq!(g.state.stand_idx, 5, "the Duck River crossing was skipped");
    assert!(
        g.outcome.as_ref().is_some_and(|e| e.won),
        "should still reach Nashville"
    );
}

#[test]
fn a_fast_final_day_on_foot_still_faces_the_duck_river_ferry() {
    let mut g = at_divide_having_vaulted_the_river(7, false);
    g.leave_stand();
    assert_eq!(g.mode, Mode::Interaction, "the ferry/ford prompt must appear");
    assert!(matches!(
        g.pending.front(),
        Some(Interaction::FerryToll { .. })
    ));
    assert!(g.outcome.is_none(), "must not win before the river is crossed");
}

#[test]
fn the_gamble_escrows_the_stake_and_pays_double_on_a_win() {
    let mut g = started(3);
    g.state.town = super::state::NATCHEZ;
    g.mode = Mode::Natchez;
    g.state.cash = 100.0;
    g.gamble(40.0);
    assert_eq!(g.state.cash, 60.0, "the stake leaves the purse when it is laid");
    assert_eq!(g.mode, Mode::Timing);
    g.resolve_timing(true, 0.9); // win
    assert_eq!(g.state.cash, 140.0, "a win returns the stake plus equal winnings");
}

#[test]
fn losing_the_gamble_keeps_the_already_escrowed_stake() {
    let mut g = started(3);
    g.state.town = super::state::NATCHEZ;
    g.mode = Mode::Natchez;
    g.state.cash = 100.0;
    g.gamble(40.0);
    g.resolve_timing(false, 0.9); // clean loss (accuracy >= 0.3, no cutpurse)
    assert_eq!(g.state.cash, 60.0, "a loss simply keeps the escrowed stake");
}

// ===== Failure-branch outcome pins =====
//
// The golden trace exercises most paths, but the bandit-robbery and drowning
// branches turn on specifics (cash fraction, health delta, robbed flag, death)
// that deserve exact assertions. These pin the arithmetic so the Stage 4 port of
// outcomes to data effect-lists can't silently drift them.

fn on_the_trace(seed: u64) -> Game {
    let mut g = started(seed);
    g.state.town = super::state::NATCHEZ;
    g.mode = Mode::Natchez;
    g.set_out_on_trace();
    g
}

#[test]
fn mason_robbery_takes_two_fifths_of_cash_ungrouped() {
    let mut g = on_the_trace(1);
    g.state.cash = 100.0;
    g.state.health = 50.0;
    g.state.grouped = false;
    g.begin_quick(super::tasks::QuickTask::Mason);
    g.resolve_quick(2.0, false); // a clean miss (not slow)
    assert!(g.state.robbed);
    assert_eq!(g.state.cash, 60.0, "loses floor(100*0.4)=40");
    assert_eq!(g.state.health, 35.0, "a clean miss costs 15 health");
    assert!(g.outcome.is_none());
}

#[test]
fn harpe_robbery_takes_three_fifths_of_cash_when_slow() {
    let mut g = on_the_trace(1);
    g.state.cash = 100.0;
    g.state.health = 80.0;
    g.state.grouped = false;
    g.begin_quick(super::tasks::QuickTask::Harpe);
    g.resolve_quick(1.5, true); // a hit, but slow
    assert!(g.state.robbed);
    assert_eq!(g.state.cash, 40.0, "loses floor(100*0.6)=60");
    assert_eq!(g.state.health, 65.0, "a slow draw costs 15 health vs the Harpes");
}

#[test]
fn a_botched_mason_draw_can_kill() {
    let mut g = on_the_trace(1);
    g.state.cash = 100.0;
    g.state.health = 10.0;
    g.begin_quick(super::tasks::QuickTask::Mason);
    g.resolve_quick(2.0, false);
    assert!(g
        .outcome
        .as_ref()
        .is_some_and(|e| e.cause_kind == super::state::GameOverCause::BanditMurder));
}

#[test]
fn fording_the_duck_river_badly_drowns_you() {
    let mut g = on_the_trace(1);
    g.begin_steady(super::tasks::SteadyTask::DuckFord);
    g.resolve_steady(false, 0.1); // not steady, well under the 0.2 catastrophe line
    assert!(g
        .outcome
        .as_ref()
        .is_some_and(|e| e.cause_kind == super::state::GameOverCause::Drowned));
}

#[test]
fn pirates_boarding_costs_reputation_and_cargo() {
    let mut g = started(1);
    g.build(BoatKind::Flatboat, 3).unwrap();
    g.buy(0, g.max_buy(0)); // a hold of corn to lose
    let before = g.state.hold[0];
    g.state.grouped = false;
    g.begin_quick(super::tasks::QuickTask::Pirates);
    g.resolve_quick(2.0, false); // good=false, not slow → 30% boarded
    assert_eq!(g.state.reputation, -10.0);
    assert!(g.state.hold[0] < before, "cargo was taken");
    assert!(!g.state.robbed, "river pirates don't set the Trace-robbed flag");
}

// ===== Declarative catastrophe selection =====
//
// `resolve_tier` is the single seam that associates any minigame outcome with a
// catastrophe band. No live kaintuck outcome declares one for a non-steady kind
// (so the golden trace can't exercise it), so pin the selection logic directly —
// including that `needs_unsteady` works uniformly via `base == Success`, not just
// for the steady-hand kind.

#[test]
fn a_declared_catastrophe_band_overrides_any_base_tier() {
    use super::tasks::resolve_tier;
    use trail_kit::{Outcome, Tier};

    let banded = Outcome {
        id: "x".into(),
        catastrophe_below: Some(0.25),
        catastrophe_needs_unsteady: false,
        success: vec![],
        partial: vec![],
        fail: vec![],
        catastrophe: vec![],
    };
    // Low quality trips the catastrophe even when the base tier would be a Fail.
    assert_eq!(resolve_tier(&banded, Tier::Fail, 0.10), Tier::Catastrophe);
    // ...and even when the base tier is Success, with no needs-unsteady refinement.
    assert_eq!(resolve_tier(&banded, Tier::Success, 0.10), Tier::Catastrophe);
    // Adequate quality keeps the base tier.
    assert_eq!(resolve_tier(&banded, Tier::Success, 0.90), Tier::Success);

    // No declared band: the base tier always stands.
    let unbanded = Outcome {
        catastrophe_below: None,
        ..banded.clone()
    };
    assert_eq!(resolve_tier(&unbanded, Tier::Fail, 0.0), Tier::Fail);

    // `needs_unsteady`: a Success-tier ("clean") run is spared even below the
    // line; a Partial/Fail ("unsteady") one is not — and this holds for ANY kind,
    // since `clean` is derived from `base == Success`, not a steady-only flag.
    let unsteady_only = Outcome {
        catastrophe_needs_unsteady: true,
        ..banded.clone()
    };
    assert_eq!(resolve_tier(&unsteady_only, Tier::Success, 0.1), Tier::Success);
    assert_eq!(resolve_tier(&unsteady_only, Tier::Partial, 0.1), Tier::Catastrophe);
    assert_eq!(resolve_tier(&unsteady_only, Tier::Fail, 0.1), Tier::Catastrophe);
}

#[test]
fn minigame_inverse_map_round_trips() {
    // begin_minigame_for (id → task) and MiniTask::outcome_id (task → id) are
    // inverses; a drift between them would silently break a hazard (panic) or an
    // empty minigame screen. Pin them in agreement for every hazard minigame.
    for id in [
        "sandbar", "falls-run", "swamp", "duck-ford", "pirates", "mason", "harpe", "side-trail",
        "dose", "patch", "bail",
    ] {
        let mut g = Game::new(0);
        g.begin_minigame_for(id);
        assert_eq!(g.pending_task.unwrap().outcome_id(), id, "round-trip for {id}");
    }
}

// ===== Golden-trace harness =====
//
// Behavior-preservation oracle for the data-driven refactor. A fully scripted,
// deterministic run across seven seeds, snapshotting the whole world state after
// every engine transition into one long string, then hashing it. The minigame /
// interaction feed is deterministic but *varied* (it cycles through success,
// partial, fail, and the occasional catastrophe) so both the winning and losing
// branches of every hazard are exercised. While the refactor preserves behavior
// the hash holds; the moment a coefficient, an RNG draw order, or a branch drifts
// the hash changes and the gate trips. Run `cargo test -p kaintuck` after every
// stage. To eyeball a drift, `cargo test -p kaintuck print_golden_trace --
// --ignored --nocapture` dumps the full trace for diffing.

/// A deterministic counter threaded through every scripted decision, so the
/// inputs depend only on the call sequence — which, as long as behavior holds,
/// is itself stable.
struct Feed {
    n: u64,
}
impl Feed {
    fn next(&mut self) -> u64 {
        let v = self.n;
        self.n = self.n.wrapping_add(1);
        v
    }
}

fn snap(g: &Game, log: &mut String) {
    use std::fmt::Write;
    let s = &g.state;
    let _ = write!(
        log,
        "{:?}|{:?}|c{:?}|d{:?}|hold{:?}|cr{}|mo{:?}|he{:?}|pr{:?}|mi{:?}|day{}|si{}|tn{}|cap{:?}|rep{:?}|rob{}|hor{}\n",
        g.phase,
        g.mode,
        s.cash,
        s.debt,
        s.hold,
        s.crew,
        s.morale,
        s.health,
        s.provisions,
        s.miles,
        s.day,
        s.stand_idx,
        s.town,
        s.credit_cap,
        s.reputation,
        s.robbed,
        s.has_horse,
    );
    // Also snapshot the narration queue (message text + cover keys). Interaction
    // derives Debug, so this folds every prose string and cover slug — the bulk
    // of the data-driven port — into the hash, closing the gap where the numeric
    // snapshot alone couldn't catch a dropped cover key or a reworded message.
    let _ = write!(log, "  q{:?}\n", g.pending);
}

/// Resolve whatever sits at the head with a varied-but-deterministic answer,
/// snapshotting after each step, until we land on a hub or an ending.
fn settle_feed(g: &mut Game, feed: &mut Feed, log: &mut String) {
    let mut guard = 0;
    loop {
        match g.mode {
            Mode::Interaction => {
                let resp = match g.pending.front() {
                    Some(Interaction::Message { .. }) => Response::Acknowledge,
                    Some(Interaction::CrewLeaves { .. }) => {
                        if feed.next() % 2 == 0 {
                            Response::Yes
                        } else {
                            Response::No
                        }
                    }
                    Some(Interaction::FerryToll { .. }) => {
                        if feed.next() % 2 == 0 {
                            Response::Yes
                        } else {
                            Response::No
                        }
                    }
                    None => Response::Acknowledge,
                };
                g.resolve(resp);
            }
            Mode::Steady => {
                let (steady, acc) = match feed.next() % 5 {
                    0 => (true, 0.95),
                    1 => (false, 0.55),
                    2 => (true, 0.80),
                    3 => (false, 0.30),
                    _ => (false, 0.12), // catastrophe for the falls/ford set-pieces
                };
                g.resolve_steady(steady, acc);
            }
            Mode::Quick => {
                let (react, hit) = match feed.next() % 4 {
                    0 => (0.5, true),
                    1 => (1.4, true), // a hit, but slow
                    2 => (2.0, false),
                    _ => (0.8, true),
                };
                g.resolve_quick(react, hit);
            }
            Mode::Crowd => {
                let (cleared, acc) = match feed.next() % 3 {
                    0 => (true, 0.9),
                    1 => (false, 0.35),
                    _ => (true, 0.7),
                };
                g.resolve_crowd(cleared, acc);
            }
            Mode::Timing => {
                let (hit, acc) = match feed.next() % 4 {
                    0 => (true, 0.9),
                    1 => (false, 0.5), // a plain loss
                    2 => (false, 0.1), // the cutpurse loss
                    _ => (true, 0.6),
                };
                g.resolve_timing(hit, acc);
            }
            Mode::Sequence => {
                let (prefix, len, perfect) = match feed.next() % 3 {
                    0 => (4, 4, true),
                    1 => (2, 4, false),
                    _ => (3, 4, false),
                };
                g.resolve_sequence(prefix, len, perfect);
            }
            Mode::Brigade => {
                let (contained, leaked, cap) = match feed.next() % 3 {
                    0 => (true, 0, 9),
                    1 => (false, 5, 9),
                    _ => (false, 2, 9),
                };
                g.resolve_brigade(contained, leaked, cap);
            }
            _ => return,
        }
        snap(g, log);
        guard += 1;
        assert!(guard < 4000, "settle_feed never converged (mode {:?})", g.mode);
    }
}

/// One full scripted playthrough for `seed`, appended to `log`.
fn golden_run(seed: u64, log: &mut String) {
    use std::fmt::Write;
    let mut feed = Feed { n: seed.wrapping_mul(2_654_435_761) };
    let mut g = started(seed);
    let _ = write!(log, "=== seed {seed} ===\n");

    g.build(BoatKind::Flatboat, 3).unwrap();
    g.buy(1, g.max_buy(1) / 2); // half-hold of whiskey
    g.buy(0, g.max_buy(0)); // top off with corn
    snap(&g, log);

    let mut natchez_first = true;
    let mut guard = 0;
    loop {
        settle_feed(&mut g, &mut feed, log);
        match g.mode {
            Mode::Pittsburgh | Mode::Town => g.depart(),
            Mode::Falls => match feed.next() % 3 {
                0 => g.falls_pilot(8.0),
                1 => g.falls_run(),
                _ => g.falls_wait(),
            },
            Mode::Natchez => {
                if natchez_first {
                    natchez_first = false;
                    g.sell_boat();
                    if feed.next() % 2 == 0 {
                        g.buy_horse(12.0);
                    }
                    if g.state.cash > 5.0 && feed.next() % 2 == 0 {
                        g.gamble(5.0);
                    } else {
                        g.set_out_on_trace();
                    }
                } else {
                    g.set_out_on_trace();
                }
            }
            Mode::TraceHub => {
                g.set_pace(if feed.next() % 2 == 0 {
                    super::state::Pace::Steady
                } else {
                    super::state::Pace::Hard
                });
                g.set_grouped(feed.next() % 2 == 0);
                g.travel_day();
            }
            Mode::Stand => {
                if feed.next() % 2 == 0 {
                    g.rest_and_resupply(8.0);
                }
                if feed.next() % 3 == 0 {
                    g.buy_horse(14.0);
                }
                g.leave_stand();
            }
            Mode::GameOver => break,
            other => panic!("seed {seed}: unexpected mode {other:?}"),
        }
        snap(&g, log);
        guard += 1;
        assert!(guard < 600, "seed {seed}: run never ended (mode {:?})", g.mode);
    }
    let _ = write!(log, "outcome {:?}\n", g.outcome);
}

/// The full deterministic trace across every seed.
fn golden_trace() -> String {
    let mut log = String::new();
    for seed in [11u64, 22, 33, 44, 55, 66, 77] {
        golden_run(seed, &mut log);
    }
    log
}

fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[test]
fn golden_trace_is_stable() {
    let trace = golden_trace();
    let got = fnv1a(&trace);
    // Baseline captured before the data-driven refactor, re-pinned when the
    // lower-river leg into Natchez got its own per-leg hazard table (heavier
    // piracy). Behavior must not drift; if this trips, run `print_golden_trace`
    // (below) before and after to diff.
    const EXPECTED: u64 = 0xff6b_255a_0ddc_cba2;
    assert_eq!(
        got, EXPECTED,
        "golden trace drifted: got {:#018x} over {} bytes",
        got,
        trace.len()
    );
}

#[test]
#[ignore = "diagnostic: dumps the full golden trace for diffing"]
fn print_golden_trace() {
    print!("{}", golden_trace());
}

// ===== Scenario / legacy-const parity =====
//
// The embedded RON scenario must reproduce, value for value, the hand-written
// tables it replaces. This is the gate for Stage 1: while logic still lives in
// Rust, the data is proven to be a faithful mirror, so later stages can repoint
// reads at the scenario with confidence.

#[test]
fn scenario_parses() {
    let _ = super::scenario_data::scenario();
}

/// The scenario is internally consistent and lines up with the engine's
/// structural constants. (The old const tables it was ported from are gone; the
/// scenario is now the single source of truth, so behaviour is locked by the
/// golden trace and the failure-branch pins, while this guards the data shape.)
#[test]
fn scenario_is_self_consistent() {
    use super::state::{
        BoatKind, GameOverCause, CINCINNATI, GOOD_NAMES, GOOD_UNITS, MEMPHIS, NUM_GOODS,
        NUM_RIVER_TOWNS, TOWN_SLUGS,
    };
    let sc = super::scenario_data::scenario();

    // Goods and towns match the engine's index constants.
    assert_eq!(sc.goods.len(), NUM_GOODS);
    for (i, g) in sc.goods.iter().enumerate() {
        assert_eq!(g.name, GOOD_NAMES[i]);
        assert_eq!(g.units, GOOD_UNITS[i]);
    }
    assert_eq!(sc.river.towns.len(), NUM_RIVER_TOWNS);
    assert_eq!(sc.boats.len(), BoatKind::ALL.len());
    for (i, t) in sc.river.towns.iter().enumerate() {
        assert_eq!(t.slug, TOWN_SLUGS[i], "town {i} slug");
        assert_eq!(t.base_ranks.len(), NUM_GOODS, "town {i} ranks width");
        assert_eq!(t.moneylender, i == CINCINNATI || i == MEMPHIS, "town {i} lender");
    }

    // Landings and stands run strictly downstream / up-trail.
    for w in sc.river.towns.windows(2) {
        assert!(w[0].milepost < w[1].milepost, "towns out of order");
    }
    for w in sc.trace.stands.windows(2) {
        assert!(w[0].milepost < w[1].milepost, "stands out of order");
    }

    // Whiskey more than doubles in value by Natchez (the trade's whole point).
    let pgh = sc.river.towns.first().unwrap().base_ranks[1];
    let nat = sc.river.towns.last().unwrap().base_ranks[1];
    assert!(nat > pgh * 2);

    // Rank tiers descend; every ending and outcome the engine can name exists.
    for w in sc.scoring.ranks.windows(2) {
        assert!(w[0].min > w[1].min, "rank tiers must descend");
    }
    for c in [
        GameOverCause::BoatWrecked,
        GameOverCause::Drowned,
        GameOverCause::BanditMurder,
        GameOverCause::Disease,
        GameOverCause::Starved,
        GameOverCause::LostInWoods,
        GameOverCause::Victory,
    ] {
        let e = sc.ending(c.key()).unwrap_or_else(|| panic!("no ending for {c:?}"));
        let should_win = matches!(c, GameOverCause::Victory);
        assert_eq!(e.won, should_win, "ending {c:?} won-flag");
    }
    for id in [
        "sandbar", "falls-run", "swamp", "duck-ford", "pirates", "mason", "harpe", "side-trail",
        "dose", "patch", "bail",
    ] {
        assert!(sc.outcome(id).is_some(), "missing outcome {id}");
    }

    // Every minigame (including the hand-coded gamble) has params of the right kind.
    use trail_kit::MiniParams;
    for (id, kind) in [
        ("sandbar", "steady"),
        ("falls-run", "steady"),
        ("swamp", "steady"),
        ("duck-ford", "steady"),
        ("pirates", "quick"),
        ("mason", "quick"),
        ("harpe", "quick"),
        ("side-trail", "crowd"),
        ("dose", "timing"),
        ("gamble", "timing"),
        ("patch", "sequence"),
        ("bail", "brigade"),
    ] {
        let p = sc
            .minigame_params(id)
            .unwrap_or_else(|| panic!("no params for {id}"));
        let actual = match p {
            MiniParams::Steady { .. } => "steady",
            MiniParams::Quick { .. } => "quick",
            MiniParams::Timing { .. } => "timing",
            MiniParams::Crowd { .. } => "crowd",
            MiniParams::Sequence { .. } => "sequence",
            MiniParams::Brigade { .. } => "brigade",
        };
        assert_eq!(actual, kind, "minigame {id} kind");
    }

    // `DieIfDead` reads the live health AFTER preceding effects, so every
    // health-affecting effect must come before it in a tier's list; otherwise a
    // fatal blow would be checked against stale health and silently not kill.
    // Enforce that no AdjustHealth follows a death effect within any tier.
    use trail_kit::Effect;
    for o in &sc.outcomes {
        for tier in [&o.success, &o.partial, &o.fail, &o.catastrophe] {
            let mut saw_death = false;
            for e in tier {
                match e {
                    Effect::Die(_) | Effect::DieIfDead(_) => saw_death = true,
                    Effect::AdjustHealth(_) => assert!(
                        !saw_death,
                        "outcome {}: AdjustHealth after a death effect (health check would be stale)",
                        o.id
                    ),
                    _ => {}
                }
            }
        }
    }

    // Each hazard table has one arm per threshold plus the clean leg, and every
    // minigame arm names an outcome that exists.
    use trail_kit::HazardArm;
    fn check_arm(sc: &trail_kit::Scenario, arm: &HazardArm) {
        match arm {
            HazardArm::Minigame { outcome, .. } => {
                assert!(sc.outcome(outcome).is_some(), "arm names missing outcome {outcome}");
            }
            HazardArm::Branch { past_divide, before } => {
                check_arm(sc, past_divide);
                check_arm(sc, before);
            }
            HazardArm::Clean { .. } | HazardArm::Special(_) => {}
        }
    }
    for haz in [&sc.river.hazards, &sc.trace.hazards] {
        assert_eq!(haz.arms.len(), haz.thresholds.len() + 1, "arms = thresholds + clean leg");
        for arm in &haz.arms {
            check_arm(sc, arm);
        }
    }

    // Every set-piece menu option names a known action, and the costs the menu
    // shows match the economy the engine actually charges.
    let known = [
        "falls-pilot", "falls-run", "falls-wait", "sell-cargo", "sell-boat", "gamble",
        "buy-horse", "set-out", "rest", "leave",
    ];
    for menu in [&sc.menus.falls, &sc.menus.natchez, &sc.menus.stand] {
        for opt in &menu.options {
            assert!(known.contains(&opt.action.as_str()), "unknown action {}", opt.action);
        }
    }
    // Per-option prices live once on the menu (the engine charges them through
    // the action's cost); pin them to their design values.
    let cost_of = |menu: &trail_kit::SetPiece<trail_kit::Gate>, action: &str| -> f64 {
        menu.options.iter().find(|o| o.action == action).unwrap().cost
    };
    assert_eq!(cost_of(&sc.menus.falls, "falls-pilot"), 8.0);
    assert_eq!(cost_of(&sc.menus.natchez, "buy-horse"), 12.0);
    assert_eq!(cost_of(&sc.menus.stand, "rest"), 8.0);
    assert_eq!(cost_of(&sc.menus.stand, "buy-horse"), 14.0);
}

#[test]
fn run_set_piece_dispatches_to_the_engine_op() {
    // The pilot fee is charged through the dispatch just as the button did.
    let mut g = started(1);
    g.build(BoatKind::Flatboat, 2).unwrap();
    g.mode = Mode::Falls;
    g.state.cash = 100.0;
    g.run_set_piece("falls-pilot", 8.0);
    assert_eq!(g.state.cash, 92.0, "pilot fee spent via dispatch");

    // A horse is bought at the option's cost.
    let mut g = started(2);
    g.state.town = super::state::NATCHEZ;
    g.mode = Mode::Natchez;
    g.state.cash = 50.0;
    g.run_set_piece("buy-horse", 12.0);
    assert!(g.state.has_horse);
    assert_eq!(g.state.cash, 38.0);

    // UI-navigation tags are engine no-ops (the screen handles them).
    let mut g = started(3);
    g.state.town = super::state::NATCHEZ;
    g.mode = Mode::Natchez;
    let cash = g.state.cash;
    g.run_set_piece("sell-cargo", 0.0);
    assert_eq!(g.state.cash, cash);
    assert_eq!(g.mode, Mode::Natchez, "engine dispatch left navigation to the UI");
}

