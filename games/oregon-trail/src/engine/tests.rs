//! Engine tests: spec-anchored unit checks, a multi-seed scripted smoke test
//! that plays whole journeys through the public API, and a behavioral check
//! that good play beats bad play.

use super::interaction::{Response, Tactic};
use super::state::{EatLevel, Mode};
use super::*;

fn fresh(seed: u64) -> Game {
    let mut g = Game::new(seed);
    g.begin("The Bierman Party".into(), 1);
    g
}

#[test]
fn outfitting_validates_and_converts() {
    let mut g = fresh(1);
    // Oxen must be 200..=300.
    assert!(g.outfit(199.0, 100.0, 50.0, 20.0, 20.0).is_err());
    assert!(g.outfit(301.0, 100.0, 50.0, 20.0, 20.0).is_err());
    // Can't overspend $700.
    assert!(g.outfit(300.0, 300.0, 100.0, 100.0, 100.0).is_err());
    // A clean buy: $1 ammo = 50 bullets; leftover becomes cash.
    g.outfit(220.0, 200.0, 100.0, 50.0, 30.0).unwrap();
    assert_eq!(g.state.oxen, 220.0);
    assert_eq!(g.state.food, 200.0);
    assert_eq!(g.state.bullets, 5000.0); // 100 * 50
    assert_eq!(g.state.clothing, 50.0);
    assert_eq!(g.state.misc, 30.0);
    assert_eq!(g.state.cash, 100.0); // 700 - 600
    assert_eq!(g.mode, Mode::Trail);
    assert_eq!(g.state.turn, 0);
}

#[test]
fn mileage_uses_the_oxen_formula() {
    // M += 200 + (A-220)/5 + 10*rand, rand in [0,1). Tested on the pure step,
    // before the incident chain (which can also move you) runs.
    let mut g = fresh(7);
    g.outfit(300.0, 200.0, 50.0, 50.0, 50.0).unwrap(); // oxen 300
    let before = g.state.miles;
    g.add_fortnight_miles();
    let gained = g.state.miles - before;
    // base = 200 + (300-220)/5 = 216, plus [0,10).
    assert!(
        (216.0..226.0).contains(&gained),
        "gained {gained}, expected 216..226"
    );

    // A weaker team covers less ground than a strong one for the same roll.
    let mut weak = fresh(7);
    weak.outfit(200.0, 200.0, 50.0, 50.0, 50.0).unwrap(); // oxen 200
    weak.add_fortnight_miles();
    assert!(weak.state.miles < g.state.miles);
}

#[test]
fn hunting_needs_bullets() {
    let mut g = fresh(3);
    g.outfit(220.0, 200.0, 0.0, 50.0, 30.0).unwrap(); // 0 ammo
    assert_eq!(g.state.bullets, 0.0);
    assert!(g.choose_hunt().is_err());
}

#[test]
fn a_missed_hunt_feeds_no_one() {
    let mut g = fresh(5);
    g.outfit(220.0, 200.0, 100.0, 50.0, 30.0).unwrap(); // 5000 bullets
    let food_before = g.state.food;
    g.choose_hunt().unwrap();
    assert_eq!(g.mode, Mode::Hunt);
    g.resolve_hunt(false, 6); // ammo exhausted without a hit
    // No food gained on a miss.
    assert_eq!(g.state.food, food_before);
    // The "dinner got away" line shows first, then we move on to eating.
    assert_eq!(g.mode, Mode::Interaction);
    g.resolve(Response::Acknowledge);
    assert_eq!(g.mode, Mode::Eat);
}

#[test]
fn a_clean_one_shot_kill_is_a_big_haul() {
    let mut g = fresh(9);
    g.outfit(220.0, 50.0, 100.0, 50.0, 30.0).unwrap();
    let food_before = g.state.food;
    g.choose_hunt().unwrap();
    g.resolve_hunt(true, 1); // bagged it on the first round
    assert!(g.state.food >= food_before + 52.0);
}

#[test]
fn stale_double_taps_are_ignored() {
    // Eating twice (the screen already moved on) must not deduct food twice.
    let mut g = fresh(11);
    g.outfit(240.0, 200.0, 100.0, 50.0, 30.0).unwrap();
    g.choose_continue();
    assert_eq!(g.mode, Mode::Eat);
    g.choose_eat(EatLevel::Moderately); // travels; mode leaves Eat
    assert_ne!(g.mode, Mode::Eat);
    let food_after = g.state.food;
    g.choose_eat(EatLevel::Well); // stale repeat — guarded, no effect
    assert_eq!(g.state.food, food_after);

    // Resolving the hunt twice must not bag the quarry twice.
    let mut h = fresh(5);
    h.outfit(240.0, 200.0, 100.0, 50.0, 30.0).unwrap();
    h.choose_hunt().unwrap();
    assert_eq!(h.mode, Mode::Hunt);
    h.resolve_hunt(true, 1);
    assert_ne!(h.mode, Mode::Hunt);
    let food_then = h.state.food;
    h.resolve_hunt(true, 1); // stale repeat — guarded, no effect
    assert_eq!(h.state.food, food_then);
}

/// Drive a whole journey through the public API with a fixed strategy.
fn play(seed: u64, good: bool) -> Game {
    let mut g = Game::new(seed);
    g.begin("Party".into(), if good { 1 } else { 5 });
    if good {
        g.outfit(240.0, 350.0, 60.0, 30.0, 20.0).unwrap(); // well-stocked, total 700
    } else {
        g.outfit(200.0, 120.0, 20.0, 5.0, 5.0).unwrap(); // thin on food & clothes
    }

    let mut steps = 0;
    loop {
        steps += 1;
        assert!(steps < 100_000, "runaway loop at seed {seed}");
        match g.mode.clone() {
            Mode::GameOver => break,
            Mode::Interaction => g.resolve(Response::Acknowledge),
            Mode::Trail => {
                // Good play hunts only when food runs low; bad play never hunts.
                if good && g.state.food < 60.0 && g.state.bullets > 39.0 {
                    let _ = g.choose_hunt();
                } else {
                    g.choose_continue();
                }
            }
            Mode::Eat => {
                let want = if good {
                    EatLevel::Well
                } else {
                    EatLevel::Poorly
                };
                let lvl = if g.state.food >= want.food_cost() {
                    want
                } else {
                    EatLevel::Poorly
                };
                g.choose_eat(lvl);
            }
            Mode::Shoot => {
                let secs = if good { 0.4 } else { 3.5 };
                g.resolve_shot(secs, good);
            }
            Mode::Hunt => {
                // Good play bags it cleanly; bad play empties the bag and misses.
                g.resolve_hunt(good, if good { 1 } else { 6 });
            }
            Mode::Riders => g.resolve_tactic(Tactic::Continue),
            Mode::Fort => g.leave_fort(),
            Mode::Splash | Mode::NewGame | Mode::Outfit => {
                unreachable!("unexpected pre-game mode at seed {seed}")
            }
        }
    }
    g
}

#[test]
fn full_games_terminate_with_a_sane_report() {
    for seed in 0..40u64 {
        let g = play(seed, true);
        let end = g.outcome.expect("game ended without an outcome");
        // The journey can't run past the winter deadline.
        assert!(g.state.turn <= MAX_TURNS, "seed {seed} ran too long");
        // The final report never shows negative supplies.
        assert!(end.food >= 0 && end.bullets >= 0 && end.clothing >= 0);
        assert!(end.misc >= 0 && end.cash >= 0);
        assert!(end.miles >= 0 && end.miles <= 2040);
        assert!(!end.rank.is_empty());
    }
}

#[test]
fn good_play_outlasts_bad_play() {
    let seeds = 0..60u64;
    let mut good_total = 0i64;
    let mut bad_total = 0i64;
    let mut good_wins = 0;
    for seed in seeds {
        let good = play(seed, true);
        good_total += good.state.miles.min(2040.0) as i64;
        if good.outcome.is_some_and(|e| e.won) {
            good_wins += 1;
        }
        bad_total += play(seed, false).state.miles.min(2040.0) as i64;
    }
    assert!(
        good_total > bad_total,
        "good {good_total} should outdistance bad {bad_total}"
    );
    // The trail is winnable with good play on at least some seeds.
    assert!(good_wins >= 1, "good play never won across 60 seeds");
}
