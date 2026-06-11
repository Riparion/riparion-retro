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
    assert!((g.state.cash - (super::state::STARTING_CASH + cap)).abs() < 1e-9);
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
                Mode::Falls => g.falls_pilot(),
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
                        g.rest_and_resupply();
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
