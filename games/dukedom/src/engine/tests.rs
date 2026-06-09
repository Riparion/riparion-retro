//! Engine tests: spec-anchored unit checks against `imports/dukedom.c`'s exact
//! numbers, plus multi-seed full-game smoke tests driven through the public API.

use super::interaction::{Interaction, Response};
use super::state::{Ending, Mode};
use super::*;

fn started(seed: u64) -> Game {
    let mut g = Game::new(seed);
    g.begin("Test Duke".into());
    g
}

#[test]
fn fresh_reign_has_the_canonical_start() {
    let g = started(1);
    assert_eq!(g.state.n_p, 100);
    assert_eq!(g.state.n_l, 600);
    assert_eq!(g.state.n_g, 4177);
    assert_eq!(g.state.n_y, 1);
    assert_eq!(g.state.c1, 3.95);
    // Seeded founding-year fertility tiers sum to the starting land.
    assert_eq!(g.state.s[1] + g.state.s[2] + g.state.s[3], 600);
    assert_eq!([g.state.s[1], g.state.s[2], g.state.s[3]], [216, 200, 184]);
    assert_eq!(g.mode, Mode::YearReport);
    let r = g.last_report.as_ref().unwrap();
    assert_eq!((r.year, r.peasants, r.land, r.grain), (1, 100, 600, 4177));
}

/// Advance a fresh game to the first Feed screen (year 1 has K=0, so no
/// double-tax prompt intervenes).
fn at_feed(seed: u64) -> Game {
    let mut g = started(seed);
    g.advance_from_report();
    assert_eq!(g.mode, Mode::Feed);
    g
}

#[test]
fn underfeeding_stirs_unrest_and_is_rejected() {
    let mut g = at_feed(1);
    let before = g.state.u1;
    // 10 HL/peasant is below the 11 threshold and isn't the whole granary.
    let r = g.submit_feed(10 * g.state.n_p);
    assert!(r.is_err());
    assert_eq!(g.mode, Mode::Feed);
    assert_eq!(g.state.u1, before + 3);
    assert!(g.input_error.is_some());
}

#[test]
fn feeding_all_grain_bypasses_unrest() {
    let mut g = at_feed(1);
    let all = g.state.n_g;
    let r = g.submit_feed(all);
    assert!(r.is_ok());
    assert_ne!(g.mode, Mode::Feed); // accepted, moved on
}

#[test]
fn starvation_survivors_are_floor_grain_over_13() {
    let mut g = at_feed(1);
    // 12 HL/peasant: no unrest (>=11) but below survival (13) — some starve.
    g.submit_feed(1200).unwrap();
    // survivors = floor(1200 / 13) = 92, so 8 of 100 die.
    assert_eq!(g.state.n_p, 92);
}

#[test]
fn plant_respects_land_peasant_and_seed_limits() {
    let mut g = at_feed(7);
    g.submit_feed(13 * g.state.n_p).unwrap();
    // Walk to the Plant screen (skip any messages / land).
    let mut steps = 0;
    while g.mode != Mode::Plant {
        steps += 1;
        assert!(steps < 50, "never reached Plant: {:?}", g.mode);
        match g.mode {
            Mode::Interaction => g.resolve(Response::Acknowledge),
            Mode::Land => {
                g.submit_buy_land(0).unwrap();
            }
            _ => panic!("unexpected mode {:?}", g.mode),
        }
    }
    let np = g.state.n_p;
    // Can't out-plant the peasant labour limit of 4 HA each.
    assert!(g.submit_plant(4 * np + 1).is_err());
    assert_eq!(g.mode, Mode::Plant);
    // Can't out-plant the land.
    assert!(g.submit_plant(g.state.n_l + 1).is_err());
}

#[test]
fn royal_tax_doubles_under_a_provoked_king() {
    // Two identical harvests, K=0 vs K=2 (double tax).
    let harvest_grain = |k: i64| {
        let mut g = Game::new(3);
        g.state.n_l = 600;
        g.state.n_g = 10_000;
        g.state.c = 0.0;
        g.state.g = [0; 11];
        g.state.k = k;
        g.run_harvest();
        g.state.n_g
    };
    // Castle base is 120; royal tax is nL/2 = 300 (K=0) or 600 (K>=2).
    assert_eq!(harvest_grain(0), 10_000 - 120 - 300);
    assert_eq!(harvest_grain(2), 10_000 - 120 - 600);
}

#[test]
fn end_checks_match_the_spec_thresholds() {
    let ending = |mutate: fn(&mut Game)| {
        let mut g = started(1);
        mutate(&mut g);
        g.check_end_of_game().map(|e| e.ending)
    };
    // Too few peasants / too little land → collapse.
    assert_eq!(ending(|g| g.state.n_p = 32), Some(Ending::Collapsed));
    assert_eq!(ending(|g| g.state.n_l = 198), Some(Ending::Collapsed));
    // Empty granary / runaway unrest → deposed.
    assert_eq!(ending(|g| g.state.n_g = 428), Some(Ending::Deposed));
    assert_eq!(ending(|g| g.state.u2 = 100), Some(Ending::Deposed));
    // Surviving past retirement with the King appeased → a win.
    assert_eq!(
        ending(|g| {
            g.state.n_y = 46;
            g.state.k = 0;
        }),
        Some(Ending::Retired)
    );
    // A healthy mid-reign duchy keeps going.
    assert_eq!(ending(|_| {}), None);
}

#[test]
fn land_buy_price_never_dips_below_four() {
    for seed in 0..40u64 {
        let mut g = started(seed);
        g.prepare_land_prices();
        assert!(g.land_buy_price >= 4, "seed {seed}: {}", g.land_buy_price);
        assert_eq!(g.land_sell_price(), g.land_buy_price - 1);
    }
}

/// A scripted player that drives a whole reign to its end.
fn auto_play(g: &mut Game, good: bool) {
    let mut steps = 0;
    while g.mode != Mode::GameOver {
        steps += 1;
        assert!(steps < 200_000, "runaway game");
        match g.mode {
            Mode::YearReport => g.advance_from_report(),
            Mode::Feed => {
                // Good: a survivable 13/peasant. Bad: a stingy 10, falling back to
                // dumping the whole granary if that's rejected.
                let want = (if good { 13 } else { 10 }) * g.state.n_p;
                let _ = g.submit_feed(want.min(g.state.n_g));
                if g.mode == Mode::Feed {
                    let _ = g.submit_feed(g.state.n_g);
                }
            }
            Mode::Land => {
                let _ = g.submit_buy_land(0);
            }
            Mode::Plant => {
                let cap = g.state.n_l.min(4 * g.state.n_p).min(g.state.n_g / 2);
                let v = if good { cap } else { cap / 3 };
                let _ = g.submit_plant(v.max(0));
            }
            Mode::Interaction => {
                let head = g.pending.front().cloned().unwrap();
                match head {
                    Interaction::Message(_) => g.resolve(Response::Acknowledge),
                    Interaction::DoubleTax => g.resolve(Response::Yes),
                    Interaction::KingLevy { .. } => g.resolve(Response::No),
                    Interaction::WarAttack => g.resolve(Response::No),
                    Interaction::WarMercenary { .. } => {
                        g.resolve(Response::Amount(if good { 20 } else { 0 }))
                    }
                }
            }
            other => panic!("unexpected mode in play: {other:?}"),
        }
    }
}

#[test]
fn full_reigns_terminate_with_a_sane_reckoning() {
    for seed in 0..40u64 {
        let mut g = started(seed);
        auto_play(&mut g, true);
        let end = g.outcome.clone().expect("reign ended");
        assert!(g.state.n_y >= 1 && g.state.n_y <= 60, "seed {seed}");
        assert!(end.score >= 0);
        assert!(!end.rank.is_empty());
        // Mode and phase are coherent at the end.
        assert_eq!(g.mode, Mode::GameOver);
        assert!(g.war.is_none(), "seed {seed} ended mid-war");
    }
}

#[test]
fn good_stewardship_outlasts_negligence() {
    let mut good_total = 0i64;
    let mut bad_total = 0i64;
    for seed in 0..60u64 {
        let mut g = started(seed);
        auto_play(&mut g, true);
        good_total += g.outcome.unwrap().years;

        let mut b = started(seed);
        auto_play(&mut b, false);
        bad_total += b.outcome.unwrap().years;
    }
    assert!(
        good_total > bad_total,
        "good {good_total} should outlast bad {bad_total}"
    );
}

#[test]
fn at_least_one_war_resumes_cleanly_across_a_save() {
    let mut saw_war = false;
    for seed in 0..40u64 {
        let mut g = started(seed);
        let mut steps = 0;
        while g.mode != Mode::GameOver {
            steps += 1;
            assert!(steps < 200_000);
            // At a war decision, prove the whole game serializes and resumes byte-identically.
            if g.mode == Mode::Interaction {
                let head = g.pending.front().cloned().unwrap();
                if matches!(head, Interaction::WarAttack | Interaction::WarMercenary { .. }) {
                    saw_war = true;
                    let json = serde_json::to_string(&g).unwrap();
                    let restored: Game = serde_json::from_str(&json).unwrap();
                    assert_eq!(g, restored, "seed {seed}: war state did not round-trip");
                }
            }
            // Advance with the same scripted policy.
            match g.mode {
                Mode::YearReport => g.advance_from_report(),
                Mode::Feed => {
                    let v = (13 * g.state.n_p).min(g.state.n_g);
                    if g.submit_feed(v).is_err() {
                        let _ = g.submit_feed(g.state.n_g);
                    }
                }
                Mode::Land => {
                    let _ = g.submit_buy_land(0);
                }
                Mode::Plant => {
                    let cap = g.state.n_l.min(4 * g.state.n_p).min(g.state.n_g / 2);
                    let _ = g.submit_plant(cap.max(0));
                }
                Mode::Interaction => {
                    let head = g.pending.front().cloned().unwrap();
                    match head {
                        Interaction::Message(_) => g.resolve(Response::Acknowledge),
                        Interaction::DoubleTax => g.resolve(Response::Yes),
                        Interaction::KingLevy { .. } => g.resolve(Response::No),
                        Interaction::WarAttack => g.resolve(Response::Yes),
                        Interaction::WarMercenary { .. } => g.resolve(Response::Amount(30)),
                    }
                }
                other => panic!("unexpected mode {other:?}"),
            }
        }
    }
    assert!(saw_war, "no war arose across 40 seeds — check the war trigger");
}
