//! Engine tests: per-formula units plus a multi-seed scripted full playthrough
//! (River → Natchez → Trace → an ending) that asserts the invariants hold.

use super::interaction::{Interaction, Response};
use super::state::{BoatKind, Mode, Phase};
use super::*;

fn started(seed: u64) -> Game {
    let mut g = Game::new(seed);
    g.begin_with("Tester".into(), ledger::Carryover::fresh());
    g
}

#[test]
fn trader_gossip_is_voiced_as_a_named_banter_message() {
    use gossip::{GossipEvent, GossipFeed, GossipKind};
    use policy::Persona;

    let mut g = started(1);
    // Offline: no feed → nothing to voice, pending stays empty.
    assert!(!g.voice_trader_gossip());
    assert!(g.pending.is_empty());

    // Attach a feed (as the multiplayer client would) and voice it.
    let mut feed = GossipFeed::default();
    feed.push(GossipEvent {
        trader: "Lemuel Boggs".into(),
        persona: Persona::Greedy,
        kind: GossipKind::ReachedNatchez,
    });
    g.gossip = Some(feed);

    assert!(g.voice_trader_gossip(), "a queued gossip event should be voiced");
    match g.pending.front() {
        Some(Interaction::Message { text, .. }) => {
            assert!(text.contains("Lemuel Boggs"), "banter didn't name the trader: {text}");
        }
        other => panic!("expected a gossip Message, got {other:?}"),
    }
    // The feed is drained — one event, one line.
    assert!(!g.voice_trader_gossip());
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
            Mode::Heave => g.resolve_heave(true, 0, 0.6),
            Mode::HotCold => g.resolve_hotcold(true, 1, 9), // a sharp find (clean)
            Mode::Hunter => g.resolve_hunter(true, 1),
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
    assert!((0.0..=100.0).contains(&s.boat_damage()), "boat damage {}", s.boat_damage());
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
                Mode::GrandTower => g.grand_tower_duck(),
                Mode::CaveInRock => g.cave_hire(5.0),
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
    // Hard pace, starving — health bleeds out within the day cap. Re-zero
    // provisions each turn so an incidental hunt-for-the-pot can't quietly feed
    // the test out of its premise.
    let mut guard = 0;
    while g.outcome.is_none() {
        g.state.provisions = 0.0;
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
fn pushing_hard_to_colberts_ferry_loses_a_day() {
    let mut g = at_divide_having_vaulted_the_river(7, false);
    g.state.pace = super::state::Pace::Hard;
    g.state.cash = 50.0;
    g.leave_stand(); // surfaces the ferry prompt
    assert!(matches!(g.pending.front(), Some(Interaction::FerryToll { .. })));
    g.resolve(Response::Yes); // pay Colbert's toll
    assert_eq!(g.state.extra_days, 1, "a hard push arrives after dark — a day lost");
}

#[test]
fn a_steady_pace_to_colberts_ferry_loses_no_day() {
    let mut g = at_divide_having_vaulted_the_river(7, false);
    g.state.pace = super::state::Pace::Steady;
    g.state.cash = 50.0;
    g.leave_stand();
    g.resolve(Response::Yes);
    assert_eq!(g.state.extra_days, 0, "an early arrival crosses without waiting");
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
fn mason_search_keeps_your_cash_on_a_clean_getaway() {
    // Reaching the hidden purse first (a sharp find) palms them a decoy — you keep
    // the lot. The signature payoff of the hide-the-money mechanic.
    let mut g = on_the_trace(1);
    g.state.cash = 100.0;
    g.state.health = 50.0;
    g.state.grouped = false;
    g.begin_hotcold(super::tasks::HotColdTask::MasonSearch);
    g.resolve_hotcold(true, 2, 7); // sharp find (2 of a 7 budget) → success
    assert!(!g.state.robbed, "a clean getaway is no robbery");
    assert_eq!(g.state.cash, 100.0, "you keep every dollar");
    assert!(g.state.reputation > 0.0);
}

#[test]
fn mason_search_costs_a_share_on_a_slow_find() {
    // Reaching the purse in the second half of the budget — they grab a share but
    // you'd cached enough to walk away with the rest. The previously-unreachable
    // middle tier (it required probes_used > the kit's full-grid par, which the
    // tight budget could never reach; grading now keys off the budget).
    let mut g = on_the_trace(1);
    g.state.cash = 100.0;
    g.state.health = 50.0;
    g.state.grouped = false;
    g.begin_hotcold(super::tasks::HotColdTask::MasonSearch);
    g.resolve_hotcold(true, 6, 7); // found, but 6 of 7 (second half) → partial
    assert!(g.state.robbed);
    assert_eq!(g.state.cash, 75.0, "loses floor(100*0.25)=25");
    assert_eq!(g.state.health, 50.0, "a slow find is no beating — morale only");
    assert!(g.outcome.is_none());
}

#[test]
fn mason_search_robs_you_when_you_dont_reach_the_purse() {
    let mut g = on_the_trace(1);
    g.state.cash = 100.0;
    g.state.health = 50.0;
    g.state.grouped = false;
    g.begin_hotcold(super::tasks::HotColdTask::MasonSearch);
    g.resolve_hotcold(false, 7, 7); // never reach it → fail tier
    assert!(g.state.robbed);
    assert_eq!(g.state.cash, 55.0, "loses floor(100*0.45)=45");
    assert_eq!(g.state.health, 38.0, "a failed scramble costs 12 health");
    assert!(g.outcome.is_none());
}

#[test]
fn harpe_fight_takes_three_fifths_of_cash_on_a_ragged_kill() {
    let mut g = on_the_trace(1);
    g.state.cash = 100.0;
    g.state.health = 80.0;
    g.state.grouped = false;
    g.begin_hunter(super::tasks::HunterTask::Harpe);
    g.resolve_hunter(true, 3); // a hit, but it emptied the gun → partial
    assert!(g.state.robbed);
    assert_eq!(g.state.cash, 40.0, "loses floor(100*0.6)=60");
    assert_eq!(g.state.health, 65.0, "a ragged kill costs 15 health vs the Harpes");
}

#[test]
fn a_failed_mason_scramble_can_kill() {
    let mut g = on_the_trace(1);
    g.state.cash = 100.0;
    g.state.health = 10.0;
    g.begin_hotcold(super::tasks::HotColdTask::MasonSearch);
    g.resolve_hotcold(false, 7, 7); // never reach it → fail; -12 health onto 10 → dead
    assert!(g
        .outcome
        .as_ref()
        .is_some_and(|e| e.cause_kind == super::state::GameOverCause::BanditMurder));
}

#[test]
fn an_empty_gun_against_the_harpes_can_kill() {
    // Surrender buys nothing with the Harpes — miss the shot and they fall on you.
    let mut g = on_the_trace(1);
    g.state.cash = 100.0;
    g.state.health = 15.0;
    g.begin_hunter(super::tasks::HunterTask::Harpe);
    g.resolve_hunter(false, 3); // missed, gun empty → fail (-20 health) → dead
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

#[test]
fn river_convoy_halves_the_pirates_take() {
    // Same seed, same boarding — sailing in company should cost less cargo,
    // because `grouped()` is true and the pirates' loss halves under it.
    let run = |convoy: bool| {
        let mut g = started(1);
        g.build(BoatKind::Flatboat, 3).unwrap();
        g.buy(0, g.max_buy(0));
        let before = g.state.hold[0];
        g.state.grouped = false;
        g.state.river_convoy = convoy;
        g.begin_quick(super::tasks::QuickTask::Pirates);
        g.resolve_quick(2.0, false); // boarded
        before - g.state.hold[0]
    };
    let solo = run(false);
    let convoyed = run(true);
    assert!(convoyed > 0, "the pirates still take something in company");
    assert!(
        convoyed < solo,
        "a convoy thins the pirates' take ({convoyed} < {solo})"
    );
}

#[test]
fn sailing_in_company_costs_a_day() {
    let mut g = started(1);
    g.build(BoatKind::Flatboat, 3).unwrap();
    g.set_river_convoy(true);
    assert_eq!(g.state.extra_days, 0);
    g.depart();
    assert_eq!(g.state.extra_days, 1, "a convoy leg burns a day forming up");
}

#[test]
fn the_river_convoy_does_not_leak_into_the_trace() {
    // Sail the last leg in company, then walk alone — the convoy's robbery
    // relief must not follow you onto the Trace (grouped() reads both flags).
    let mut g = started(1);
    g.state.town = super::state::NATCHEZ;
    g.mode = Mode::Natchez;
    g.state.river_convoy = true;
    g.set_out_on_trace();
    assert!(!g.state.river_convoy, "entering the Trace clears the river convoy");
    g.state.grouped = false;
    assert!(!g.grouped(), "walking alone on the Trace is truly alone");
}

#[test]
fn lost_days_count_toward_the_reckoning() {
    let mut g = started(1);
    let d0 = g.days_elapsed();
    g.lose_days(2);
    assert_eq!(g.days_elapsed(), d0 + 2, "extra_days folds into days elapsed");
}

#[test]
fn the_counterfeit_con_skims_the_purse() {
    let mut g = started(1);
    g.state.cash = 100.0;
    g.do_counterfeit();
    assert!(
        g.state.cash < 100.0 && g.state.cash >= 60.0,
        "8–15% skimmed, capped at $40 (cash now {})",
        g.state.cash
    );
    assert!(g.state.reputation < 0.0, "being cheated dents reputation");
    assert!(g.state.morale < 100.0, "and morale");
}

#[test]
fn the_counterfeit_con_is_a_no_op_on_an_empty_purse() {
    let mut g = started(1);
    g.state.cash = 3.0;
    g.do_counterfeit();
    assert_eq!(g.state.cash, 3.0, "nothing worth skimming, no loss");
    assert_eq!(g.state.reputation, 0.0, "and no reputation hit");
}

#[test]
fn cave_in_rock_tell_decides_whether_a_rich_boat_is_trapped() {
    // Taking the stranger's offer opens the pilot-tell (a timing read), not an
    // immediate coin-flip — skill now decides the con.
    let mut g = started(1);
    g.build(BoatKind::Flatboat, 3).unwrap();
    g.buy(0, g.max_buy(0)); // load up on corn
    g.mode = Mode::CaveInRock;
    assert!(g.state.cargo_value() > 60.0);
    g.cave_take();
    assert_eq!(g.mode, Mode::Timing, "the offer opens the pilot tell");

    // Reading the tell clean waves him off safely, even with a fat hold — no
    // boarding, no mark.
    let mut clean = g.clone();
    clean.resolve_timing(true, 0.9);
    assert_ne!(clean.mode, Mode::Quick, "a clean read is safe");
    assert!(!clean.state.crossed_mason, "no confrontation, no mark");

    // Missing the tell with a hold worth taking drops you into the boarding and
    // marks you for the Trace.
    g.resolve_timing(false, 0.1);
    assert_eq!(g.mode, Mode::Quick, "a missed tell on a rich boat is the ambush");
    assert!(g.state.crossed_mason, "tangling with Mason's gang marks him for the Trace");

    // A near-empty hold isn't worth the betrayal — even a missed tell passes you
    // through, and with no confrontation Mason leaves no mark.
    let mut g = started(1);
    g.build(BoatKind::Flatboat, 3).unwrap();
    g.mode = Mode::CaveInRock;
    assert!(g.state.cargo_value() <= 60.0);
    g.cave_take();
    g.resolve_timing(false, 0.1);
    assert_ne!(g.mode, Mode::Quick, "a poor boat is waved through");
    assert!(!g.state.crossed_mason, "no confrontation, no mark");
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
        "sandbar", "cordelle", "falls-run", "cave-run", "swamp", "duck-ford", "pirates", "mason",
        "harpe", "trace-hunt", "side-trail", "dose", "patch", "bail", "self-repair",
    ] {
        let mut g = Game::new(0);
        g.begin_minigame_for(id);
        assert_eq!(g.pending_task.unwrap().outcome_id(), id, "round-trip for {id}");
    }
}

// ===== Boat damage & repair =====

use super::tasks::{HeaveTask, SequenceTask};

/// A flatboat built and ready, parked at a river town hub, for repair tests.
fn afloat_at_town(seed: u64, town: usize, damage: f64) -> Game {
    let mut g = started(seed);
    g.build(BoatKind::Flatboat, 2).unwrap();
    g.state.boat.as_mut().unwrap().damage = damage;
    g.state.town = town;
    g.mode = Mode::Town;
    g
}

#[test]
fn a_hazard_inflicts_hull_damage() {
    let mut g = started(1);
    g.build(BoatKind::Flatboat, 2).unwrap();
    assert_eq!(g.state.boat_damage(), 0.0);
    // A grounding that doesn't come off clean (Partial) splinters the hull.
    g.begin_heave(HeaveTask::Ground);
    g.resolve_heave(false, 0, 0.3);
    assert!(g.state.boat_damage() > 0.0, "sandbar partial should damage the hull");
}

#[test]
fn enough_damage_wrecks_her() {
    let mut g = started(1);
    g.build(BoatKind::Flatboat, 2).unwrap();
    g.state.boat.as_mut().unwrap().damage = 95.0;
    // The sandbar partial adds another dozen points — over 100, she goes down.
    g.begin_heave(HeaveTask::Ground);
    g.resolve_heave(false, 0, 0.0);
    assert_eq!(g.state.boat_damage(), 100.0);
    assert_eq!(
        g.outcome.as_ref().map(|e| e.cause_kind),
        Some(state::GameOverCause::BoatWrecked)
    );
}

#[test]
fn a_battered_hull_seeps_cargo_each_leg() {
    let mut g = started(1);
    g.build(BoatKind::Flatboat, 2).unwrap();
    g.state.hold[0] = 100;
    // Below the threshold she holds tight.
    g.state.boat.as_mut().unwrap().damage = 30.0;
    g.apply_hull_seepage();
    assert_eq!(g.state.hold[0], 100, "no seepage below the threshold");
    // Badly hurt, she weeps.
    g.state.boat.as_mut().unwrap().damage = 80.0;
    g.apply_hull_seepage();
    assert!(g.state.hold[0] < 100, "a battered hull should leak cargo");
}

#[test]
fn the_boatwright_mends_her_for_a_scaling_fee() {
    let mut g = afloat_at_town(2, state::LOUISVILLE, 50.0);
    assert!(g.is_boatyard());
    let cost = g.repair_cost();
    assert_eq!(cost, 45.0, "50 damage * $0.9/pt, rounded up");
    let spent_before = g.state.cash + g.state.debt;
    g.repair_in_port();
    assert_eq!(g.state.boat_damage(), 0.0, "she's sound again");
    assert_eq!(g.state.cash + g.state.debt - spent_before, cost, "charged the fee");
}

#[test]
fn the_boatwright_only_works_at_a_boatyard() {
    let mut g = afloat_at_town(2, 1, 50.0); // Wheeling — no boatyard
    assert!(!g.is_boatyard());
    g.repair_in_port();
    assert_eq!(g.state.boat_damage(), 50.0, "no boatyard, no repair");
}

#[test]
fn self_repair_costs_days_and_runs_the_sequence() {
    let mut g = afloat_at_town(7, 1, 50.0); // self-repair is offered anywhere
    let days0 = g.state.extra_days;
    g.self_repair();
    assert_eq!(
        g.state.extra_days,
        days0 + scenario().repair.self_repair_days,
        "self-repair burns days"
    );
    // The moored hazard's narration drains, then the sequence launches; a clean
    // run (settle plays it perfect) mends her whole.
    settle(&mut g);
    assert_eq!(g.state.boat_damage(), 0.0, "a perfect self-repair mends her whole");
}

#[test]
fn self_repair_perfect_partial_and_bomb() {
    // Perfect → fully mended.
    let mut g = afloat_at_town(3, 1, 60.0);
    g.begin_sequence(SequenceTask::SelfRepair);
    g.resolve_sequence(6, 6, true);
    assert_eq!(g.state.boat_damage(), 0.0);

    // A slip → mended in proportion to how far you got (3/6 of the damage).
    let mut g = afloat_at_town(3, 1, 60.0);
    g.begin_sequence(SequenceTask::SelfRepair);
    g.resolve_sequence(3, 6, false);
    assert!((g.state.boat_damage() - 30.0).abs() < 1e-9, "half-done halves the damage");

    // A botched run (no step right) → worse than before, by the bomb penalty.
    let mut g = afloat_at_town(3, 1, 60.0);
    g.begin_sequence(SequenceTask::SelfRepair);
    g.resolve_sequence(0, 6, false);
    assert_eq!(g.state.boat_damage(), 60.0 + scenario().repair.bomb_penalty);
}

#[test]
fn a_botched_self_repair_can_sink_a_failing_hull() {
    // The wreck-at-100 rule must hold on the hand-coded bomb path too, not just
    // the AdjustBoatDamage effect path (the shared adjust_boat_damage chokepoint).
    let mut g = afloat_at_town(3, 1, 95.0);
    g.begin_sequence(SequenceTask::SelfRepair);
    g.resolve_sequence(0, 6, false); // bomb adds bomb_penalty, tipping past 100
    assert_eq!(g.state.boat_damage(), 100.0);
    assert_eq!(
        g.outcome.as_ref().map(|e| e.cause_kind),
        Some(state::GameOverCause::BoatWrecked)
    );
}

#[test]
fn self_repair_sequence_grows_with_the_damage() {
    use trail_kit::MiniParams;
    let base = match scenario().minigame_params("self-repair").unwrap() {
        MiniParams::Sequence { length, .. } => *length,
        _ => unreachable!("self-repair is a sequence"),
    };
    let max = scenario().repair.self_seq_max_len;
    // Off the self-repair task there is no override.
    let g = afloat_at_town(3, 1, 0.0);
    assert_eq!(g.self_repair_seq_len(), None);
    // A sound-ish hull plays the base length; a wreck the max.
    let mut g = afloat_at_town(3, 1, 0.0);
    g.begin_sequence(SequenceTask::SelfRepair);
    assert_eq!(g.self_repair_seq_len(), Some(base));
    g.state.boat.as_mut().unwrap().damage = 100.0;
    assert_eq!(g.self_repair_seq_len(), Some(max));
}

#[test]
fn damage_cuts_the_salvage_value() {
    use super::state::Boat;
    let sound = Boat::new(BoatKind::Flatboat).salvage_value();
    assert_eq!(sound, Boat::new(BoatKind::Flatboat).lumber_value(), "sound = full lumber");
    let mut wrecked = Boat::new(BoatKind::Flatboat);
    wrecked.damage = 100.0;
    assert!(wrecked.salvage_value() < sound, "a wreck fetches less for lumber");
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
            // Every paused minigame resolves through the SAME outcome-band tables
            // the bots use (crate::policy::minigame_action), single-sourced so the
            // oracle and the bot stressor can't drift. One `feed.next()` per step,
            // exactly as before, so the golden hash is unchanged.
            Mode::Steady
            | Mode::Quick
            | Mode::Crowd
            | Mode::Timing
            | Mode::Sequence
            | Mode::Brigade
            | Mode::Heave
            | Mode::HotCold
            | Mode::Hunter => {
                let action = crate::policy::minigame_action(g, feed.next())
                    .expect("a paused minigame mode always yields a resolution");
                g.apply(action);
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
            Mode::GrandTower => {
                if feed.next() % 2 == 0 {
                    g.grand_tower_treat(2.0)
                } else {
                    g.grand_tower_duck()
                }
            }
            Mode::CaveInRock => match feed.next() % 3 {
                0 => g.cave_take(),
                1 => g.cave_hire(5.0),
                _ => g.cave_run(),
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
    // Shared FNV-1a (same algorithm as before — golden hash unchanged).
    retro_core::hash::fnv1a(s.as_bytes())
}

#[test]
fn golden_trace_is_stable() {
    let trace = golden_trace();
    let got = fnv1a(&trace);
    // Baseline captured before the data-driven refactor, re-pinned for: the
    // lower-river leg into Natchez getting its own hazard table (heavier piracy),
    // then ambient crew banter replacing the flat clean-leg lines (narrative
    // only), then a boatmen flavor pass (more banter + folk hazard names), and
    // now four new river landings (Marietta, Maysville, Shawneetown, Grand Tower)
    // with the Grand Tower initiation set-piece — a real behavior change: more
    // legs means a longer RNG stream and more markets. Re-pinned again for the
    // river-pirates ingest (RESEARCH_PIRATES.md): six new ambient banter beats
    // (narrative), the "sail in company" river convoy (default off, so it adds no
    // draw on this trace), and the counterfeit-con Special on the lower-river leg
    // into Natchez (a new (60,64] hazard band). Re-pinned once more for the
    // deferred-items pass: the Cave-in-Rock relay-pilot set-piece (a new mandatory
    // landing at Cairo — a real behavior change, with its cargo-value trap into the
    // pirate quick-draw), plus Mason through-line beats and Colbert's-ferry copy.
    // Re-pinned for the navigation ingest (RESEARCH_NAVIGATION.md): six river legs
    // (Wheeling, Marietta, Shawneetown, Cairo, Grand Tower, Memphis) get their own
    // per-leg hazard tables — retuned band widths per the real danger of each reach
    // (upper-Ohio riffles/bars, lower-Ohio snags, confluence boils, the Grand Chain
    // ambush, the Chickasaw-bluff eddies) plus folk-named flavor messages. Retuned
    // thresholds shift which arm fires, so river outcomes/scores move (the win/loss
    // pattern across seeds is unchanged). Falls success/partial prose enriched too.
    // Re-pinned for the two-vector market pass: each town now carries a local
    // supply/demand bias (Porkopolis pork, Monongahela rye, Kentucky leaf, the
    // provision-hungry lower landings) that scales the rank-mean, plus an 8% base
    // bid/ask spread widened on thin markets — a real behavior change (every
    // buy/sell quote moves, so cash, holds, reputation and downstream scores all
    // drift). Adds the engine-derived "wharf factor" reality line at each biased
    // dock and a "rumor-river-prices" thesis beat early on the upper Ohio.
    // Re-pinned for the minigame redesign (TODO_KAINTUCK_MINIGAMES.md): grounding
    // moved from a steady-hand trace to a press-and-hold Heave (crew/draft levers);
    // the swamp from Steady to a HotCold route-find; Mason from a quick-draw to a
    // HotCold scramble for your hidden purse (hide-the-money); the Harpes from a
    // quick-draw to a Hunter fight (surrender buys nothing); the river pirates kept
    // Quick but with their own boarding words; Cave-in-Rock's "take the offer" now
    // opens a Timing pilot-tell before the cargo-value boarding (skill, not a coin
    // flip); plus two net-adds — a Heave cordelle band on the Grand Tower leg and a
    // Hunter "hunt for the pot" band on the Trace. New minigame outcomes/effects
    // and two new hazard bands shift the RNG stream and every downstream score.
    // Re-pinned again for the HotCold grading fix: the search now grades against
    // the encounter's budget (max_probes) instead of the kit's lenient full-grid
    // par, so Mason's "slow find" Partial tier (lose a share) is reachable where
    // before every find took Success — a real behavior change on the Trace search.
    // Re-pinned for moving the swamp from a HotCold probe to a Crowd route-memory
    // thread (pathfinding the firm line through the mud): the swamp success prose
    // changed and its penalty moved to the Fail tier, drift-scaling the lost miles.
    // Re-pinned for folding reputation into the boatyard credit: the per-leg
    // interest rate now swings ±2.5 points around 5% with the trader's standing
    // (cheaper when respected, dearer when notorious), and the Cincinnati/Memphis
    // credit cap scales the cash-multiple ±50% by reputation — so debt growth and
    // borrowing room (and every downstream cash/score) move with your name.
    // Re-pinned for the boat damage & repair system: river hazard outcomes
    // (sandbar, cordelle, falls/cave runs, pirates, patch, bail) now carry
    // `AdjustBoatDamage` effects, a battered hull seeps cargo each leg and adds to
    // every river mishap's severity drift, and the Natchez cash-out pays the
    // damage-discounted salvage value rather than full lumber — a real behavior
    // change (cargo, cash, and scores move once the hull takes hurt). The port
    // boatwright and the self-repair sequence are player-initiated and so don't
    // enter this scripted trace.
    // Re-pinned for the actionable-banter pass: the dock "wharf factor" reality
    // line is rewritten to carry a downstream gradient hint on the cheap good
    // ("they'll pay near double for it down at Natchez", from the RNG-free
    // rank-mean) and a hold-aware sell prompt on the dear good, with the opener
    // rotated per town. Narrative only — the diff is confined to those queued
    // message strings; no numeric/behavioral field moves (verified by diffing
    // print_golden_trace: every changed line is a wharf-opener message). Trader
    // gossip is unaffected here — it's multiplayer-only and never queued offline.
    // Re-pinned for the dockside-rumor mechanic (a real behavior change): each
    // landing now pre-rolls the NEXT town's prices into `committed_prices` (so a
    // rumor's truth is fixed before the player loads) and rolls a tip from the
    // main stream (source, the reliability gate, and — when the source lies —
    // the wrong band). Both the moved price roll and those extra draws reorder
    // the one RNG stream, so hazard arms and whole journeys shift; the run also
    // queues the tip and its next-dock payoff as messages. Verified the new
    // win/loss split across the seven seeds stays healthy (4 home / 3 lost, no
    // degenerate scores) and the trace's line *kinds* are unchanged (only more
    // of them, plus the rumor messages). Single-player play uses random seeds,
    // so no real run is "pinned" to a seed — only this fixture moves.
    // Re-pinned once more for the rumor-subject tie-break fix: `generate` now
    // resolves equally-extreme goods to the LOWEST index (matching its doc),
    // where it previously took the highest. Good selection is RNG-free, so the
    // stream and every per-seed outcome are byte-identical to the prior pin —
    // only some rumor message strings change (the good a tie names).
    // Behavior must not drift; if this trips, run `print_golden_trace` to diff.
    const EXPECTED: u64 = 0xe5e7_050c_7316_77f1;
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

/// Dev tool: summarize every leg's hazard distribution straight from the
/// scenario data — which hazards can occur on a leg and at what odds. Reads the
/// embedded `Scenario` only (no engine state, no RNG), so the percentages are
/// the band widths the selection loops in `river.rs` / `trace.rs` actually use.
/// Handy for eyeballing a per-leg override (the lower-river piracy weighting) and
/// for catching the `arms[i] <-> thresholds[i-1]` off-by-one in a fresh table.
#[test]
#[ignore = "diagnostic: prints each leg's hazard distribution from the scenario"]
fn print_leg_hazards() {
    use super::scenario_data::scenario;
    use super::state::NUM_RIVER_TOWNS;
    use trail_kit::scenario::HazardTable;

    // Per-arm probabilities from cumulative thresholds, indexed exactly as the
    // engine selects: `arms[0]` is the fall-through (clean) slice above the top
    // threshold; `arms[i+1]` owns `(thresholds[i-1], thresholds[i]]`.
    fn band_pcts(t: &HazardTable) -> Vec<f64> {
        let mut pcts = vec![0.0; t.arms.len()];
        pcts[0] = 100.0 - t.thresholds.last().copied().unwrap_or(0.0);
        let mut prev = 0.0;
        for (i, &th) in t.thresholds.iter().enumerate() {
            pcts[i + 1] = th - prev;
            prev = th;
        }
        pcts
    }

    // A Branch resolves to one side by position; everything else stands alone.
    fn resolve(arm: &HazardArm, past_divide: bool) -> &HazardArm {
        match arm {
            HazardArm::Branch {
                before,
                past_divide: pd,
            } => {
                if past_divide {
                    pd
                } else {
                    before
                }
            }
            other => other,
        }
    }

    fn label(arm: &HazardArm) -> String {
        match arm {
            HazardArm::Clean { .. } => "clean".into(),
            HazardArm::Minigame { outcome, .. } => outcome.clone(),
            HazardArm::Special(s) => format!("{s} (special)"),
            HazardArm::Branch {
                before,
                past_divide,
            } => format!("{} / {}", label(before), label(past_divide)),
        }
    }

    // Print arms as a distribution, descending by odds, resolving any branch.
    fn print_table(t: &HazardTable, past_divide: bool) {
        let pcts = band_pcts(t);
        let mut rows: Vec<(String, f64)> = t
            .arms
            .iter()
            .zip(&pcts)
            .map(|(a, &p)| (label(resolve(a, past_divide)), p))
            .collect();
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (name, p) in rows {
            println!("    {name:<22}{p:>3.0}%");
        }
    }

    let sc = scenario();

    println!("\nRIVER LEGS  (one hazard roll per leg)");
    for to in 1..NUM_RIVER_TOWNS {
        let from = &sc.river.towns[to - 1].name;
        let town = &sc.river.towns[to];
        let (table, tag) = match town.hazards.as_ref() {
            Some(h) => (h, "[OVERRIDE]"),
            None => (&sc.river.hazards, "[phase-wide]"),
        };
        println!("  {from} -> {} {tag}", town.name);
        print_table(table, false);
    }

    println!("\nTRACE PHASE  (one roll per day; bandits branch at the divide)");
    let trace = &sc.trace.hazards;
    println!("  -- before the divide --");
    print_table(trace, false);
    println!("  -- past the divide (mile >= {}) --", sc.trace.divide_at);
    print_table(trace, true);
    if !trace.grouped_thins.is_empty() {
        let thinned: Vec<String> = trace
            .grouped_thins
            .iter()
            .map(|&i| label(&trace.arms[i]))
            .collect();
        println!(
            "  note: travelling grouped clears {} to a clean day ~50% of the time",
            thinned.join(", ")
        );
    }
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
        // Every local market bias names a real good and stays in the bands the
        // price math assumes: supply/spread strictly under 1.0 (so neither the
        // mid nor the bid can collapse to zero), demand non-negative.
        for b in &t.market {
            assert!(
                GOOD_NAMES.contains(&b.good.as_str()),
                "town {i} market: unknown good {:?}",
                b.good
            );
            assert!((0.0..1.0).contains(&b.supply), "town {i} {} supply", b.good);
            assert!(b.demand >= 0.0, "town {i} {} demand", b.good);
            assert!((0.0..1.0).contains(&b.spread), "town {i} {} spread", b.good);
        }
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
        "sandbar", "cordelle", "falls-run", "cave-run", "swamp", "duck-ford", "pirates", "mason",
        "harpe", "trace-hunt", "side-trail", "dose", "patch", "bail", "self-repair",
    ] {
        assert!(sc.outcome(id).is_some(), "missing outcome {id}");
    }

    // Every minigame (including the hand-coded gamble and cave-tell) has params of
    // the right kind — the kind the host's `Mode`/resolve for that id expects.
    use trail_kit::MiniParams;
    for (id, kind) in [
        ("falls-run", "steady"),
        ("cave-run", "steady"),
        ("duck-ford", "steady"),
        ("sandbar", "heave"),
        ("cordelle", "heave"),
        ("swamp", "crowd"),
        ("mason", "hotcold"),
        ("harpe", "hunter"),
        ("trace-hunt", "hunter"),
        ("pirates", "quick"),
        ("side-trail", "crowd"),
        ("dose", "timing"),
        ("gamble", "timing"),
        ("cave-tell", "timing"),
        ("patch", "sequence"),
        ("self-repair", "sequence"),
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
            MiniParams::Heave { .. } => "heave",
            MiniParams::HotCold { .. } => "hotcold",
            MiniParams::Hunter { .. } => "hunter",
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
        "falls-pilot", "falls-run", "falls-wait", "gt-treat", "gt-duck", "cave-take", "cave-hire",
        "cave-run", "sell-cargo", "sell-boat", "gamble", "moneylender", "buy-horse", "set-out",
        "rest", "leave",
    ];
    for menu in [
        &sc.menus.falls,
        &sc.menus.grandtower,
        &sc.menus.caveinrock,
        &sc.menus.natchez,
        &sc.menus.stand,
    ] {
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
    assert_eq!(cost_of(&sc.menus.grandtower, "gt-treat"), 2.0);
    assert_eq!(cost_of(&sc.menus.caveinrock, "cave-hire"), 5.0);
    assert_eq!(cost_of(&sc.menus.natchez, "buy-horse"), 12.0);
    assert_eq!(cost_of(&sc.menus.stand, "rest"), 8.0);
    assert_eq!(cost_of(&sc.menus.stand, "buy-horse"), 14.0);
}

/// The dock's *reality* line is derived from the same market data the prices use,
/// so it can never drift from them: it must name the town's actually-cheapest and
/// actually-dearest goods, and be silent only where the town has no bias.
#[test]
fn reality_banter_matches_the_market() {
    use super::river::market_reality_line;
    use super::state::{NUM_GOODS, NUM_RIVER_TOWNS};
    let sc = super::scenario_data::scenario();
    let empty = [0i64; NUM_GOODS];

    for town in 0..NUM_RIVER_TOWNS {
        let market = &sc.river.towns[town].market;
        let cheapest = market
            .iter()
            .filter(|b| b.supply > 0.0)
            .max_by(|a, b| a.supply.total_cmp(&b.supply));
        let dearest = market
            .iter()
            .filter(|b| b.demand > 0.0)
            .max_by(|a, b| a.demand.total_cmp(&b.demand));
        let line = market_reality_line(town, &empty);

        if cheapest.is_none() && dearest.is_none() {
            assert!(line.is_none(), "town {town} has no bias but speaks: {line:?}");
            continue;
        }
        let line = line.unwrap_or_else(|| panic!("town {town} has bias but is silent"));
        if let Some(b) = cheapest {
            let good = b.good.to_lowercase();
            assert!(
                line.contains(&good),
                "town {town}: cheap good {good:?} not named in {line:?}"
            );
        }
        if let Some(b) = dearest {
            let good = b.good.to_lowercase();
            assert!(
                line.contains(&good),
                "town {town}: dear good {good:?} not named in {line:?}"
            );
        }
    }

    // Concrete anchor: Cincinnati is cheap on pork (Porkopolis) and dear on hides.
    let cincy = market_reality_line(super::state::CINCINNATI, &empty).unwrap();
    assert!(cincy.contains("pork") && cincy.contains("hides"), "Cincinnati: {cincy:?}");
    // Natchez carries its premium in the distance gradient, not a local craving.
    assert!(market_reality_line(super::state::NATCHEZ, &empty).is_none());
}

/// The enriched dock line must be *actionable*: the cheap good carries a
/// downstream gradient hint (direction + magnitude), and the dear good reacts to
/// what the player is actually holding.
#[test]
fn reality_banter_is_actionable() {
    use super::river::market_reality_line;
    use super::state::{GOOD_NAMES, NUM_GOODS, PITTSBURGH};
    let empty = [0i64; NUM_GOODS];

    // Pittsburgh's cheap whiskey fetches far more downriver — the line names a
    // magnitude and points downstream ("down at <town>").
    let pitt = market_reality_line(PITTSBURGH, &empty).expect("Pittsburgh has bias");
    assert!(pitt.contains("down at "), "no downstream hint: {pitt:?}");
    assert!(
        ["near double", "half again as much", "a good deal more"]
            .iter()
            .any(|m| pitt.contains(m)),
        "no gradient magnitude: {pitt:?}"
    );

    // Cincinnati is dear on hides. Holding hides flips the line to a sell prompt
    // ("right here"); holding none leaves it as plain colour.
    let hides = GOOD_NAMES.iter().position(|n| *n == "Hides").unwrap();
    let mut hold = [0i64; NUM_GOODS];
    hold[hides] = 12;
    let with = market_reality_line(super::state::CINCINNATI, &hold).unwrap();
    let without = market_reality_line(super::state::CINCINNATI, &empty).unwrap();
    assert!(with.contains("right here"), "hold-aware prompt missing: {with:?}");
    assert!(!without.contains("right here"), "leaked sell prompt with empty hold: {without:?}");
}

/// A dockside rumor must be reproducible from the seed, and a tip "holds" exactly
/// when its claimed band equals the town's actual band — the property that makes
/// source reliability a learnable signal rather than noise.
#[test]
fn rumor_is_deterministic_and_truth_is_derived() {
    use super::rng::GameRng;
    use super::rumor::generate;

    let town = 5usize; // Louisville
    let committed = super::prices::roll_prices(town, &mut GameRng::from_seed(42));

    // Same seed → byte-identical rumor.
    let a = generate(&mut GameRng::from_seed(7), town, &committed).expect("kaintuck has sources");
    let b = generate(&mut GameRng::from_seed(7), town, &committed).unwrap();
    assert_eq!(a, b, "same seed must yield the same rumor");

    // Across many seeds: the claimed band is always 1..=3, and `held()` (the
    // derived verdict, recomputed against the committed prices) is reached both
    // ways — proving the reliability gate actually flips some tips to lies.
    let (mut saw_true, mut saw_false) = (false, false);
    for s in 0..400u64 {
        let r = generate(&mut GameRng::from_seed(s), town, &committed).unwrap();
        assert!((1..=3).contains(&r.claimed_band), "band out of range: {r:?}");
        if r.held(&committed) { saw_true = true } else { saw_false = true }
    }
    assert!(saw_true && saw_false, "both held and wind tips should occur");
}

/// Composing a tip and its payoff must resolve every placeholder and name the
/// good and town, for each band and each held/wind outcome.
#[test]
fn rumor_compose_and_resolve_are_fully_resolved() {
    use super::rumor::Rumor;
    use super::state::GOOD_NAMES;

    let good = 1usize; // Whiskey
    let town = 5usize; // Louisville
    let good_name = GOOD_NAMES[good].to_lowercase();
    let town_name = &super::scenario_data::scenario().river.towns[town].name;

    for band in 1u8..=3 {
        let r = Rumor { source: "harbormaster".into(), town, good, claimed_band: band };
        let (voice, line) = r.compose().expect("authored band phrasing");
        assert!(!voice.is_empty());
        assert!(!line.contains('{'), "unresolved placeholder: {line:?}");
        assert!(line.contains(&good_name) && line.contains(town_name.as_str()), "line: {line:?}");
        for held in [true, false] {
            let payoff = r.resolve_line(held).expect("authored confirm phrasing");
            assert!(!payoff.contains('{'), "unresolved placeholder: {payoff:?}");
            assert!(payoff.contains(&good_name) && payoff.contains(town_name.as_str()), "payoff: {payoff:?}");
        }
    }
}

/// A pre-committed price roll is installed verbatim only at the town it was rolled
/// for, then consumed; a commit tagged for a different town is dropped and prices
/// roll fresh (so a mis-routed arrival can never face the wrong town's prices).
#[test]
fn committed_prices_install_only_for_their_town() {
    use super::state::NUM_GOODS;

    let mut g = started(1);
    let stash: [f64; NUM_GOODS] = [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

    // Tagged for the current town → installed verbatim, no RNG drawn.
    g.state.committed_prices = Some((g.state.town, stash));
    let before = g.rng.clone();
    super::prices::generate_prices(&mut g.state, &mut g.rng);
    assert_eq!(g.state.prices, stash, "committed prices not installed");
    assert!(g.state.committed_prices.is_none(), "commit not consumed");
    assert_eq!(g.rng, before, "installing a commit must not consume RNG");

    // Tagged for a DIFFERENT town → dropped; prices roll fresh (RNG advances).
    g.state.committed_prices = Some((g.state.town + 3, stash));
    let before = g.rng.clone();
    super::prices::generate_prices(&mut g.state, &mut g.rng);
    assert_ne!(g.state.prices, stash, "stale-town commit must NOT be installed");
    assert!(g.state.committed_prices.is_none(), "stale commit not consumed");
    assert_ne!(g.rng, before, "a fresh roll must consume RNG");
}

/// The scenario's rumor flavor must cover every band and payoff the engine can
/// ask for, keep reliabilities in [0,1], and use only `{good}`/`{town}`. Crucially
/// it asserts every (source × band) tip and held/wind payoff actually COMPOSES,
/// so a regenerated corpus (`just gen-rumors`) that drops a band can't ship a
/// rumor the engine silently can't voice.
#[test]
fn rumor_flavor_is_complete() {
    use super::rumor::Rumor;
    let r = &super::scenario_data::scenario().rumors;
    assert!(!r.sources.is_empty(), "no rumor sources");
    for s in &r.sources {
        assert!((0.0..=1.0).contains(&s.reliability), "source {:?} reliability out of [0,1]", s.key);
        assert!(!s.voice.is_empty(), "source {:?} has no voice", s.key);
    }
    // Only {good}/{town} are filled at compose; any other brace would leak.
    for p in r.lines.iter().chain(r.confirms.iter()) {
        for t in &p.templates {
            let stripped = t.replace("{good}", "").replace("{town}", "");
            assert!(!stripped.contains('{'), "template has an unknown placeholder: {t:?}");
        }
    }
    // Every tip the engine can generate (any source, any band 1..=3) must compose,
    // and both payoff outcomes must compose — no silently-voiceless rumor.
    for s in &r.sources {
        for band in 1u8..=3 {
            let rumor = Rumor { source: s.key.clone(), town: 5, good: 1, claimed_band: band };
            assert!(rumor.compose().is_some(), "source {:?} band {band} has no tip phrasing", s.key);
            assert!(rumor.resolve_line(true).is_some(), "no 'held' payoff phrasing");
            assert!(rumor.resolve_line(false).is_some(), "no 'wind' payoff phrasing");
        }
    }
}

/// The ask must never sit below the bid in the *actual* transaction path. This
/// drives the real `Game::buy_price`/`sell_price` (not a re-derived inequality),
/// so a sign error or a dropped spread term in either method trips it — and the
/// bid must stay strictly positive so a sale never pays the player nothing.
#[test]
fn buy_ask_never_below_sell_bid() {
    use super::state::{NUM_GOODS, NUM_RIVER_TOWNS};
    let mut g = started(1);
    g.state.reputation = 0.0; // neutral: only the spread separates ask and bid
    for town in 0..NUM_RIVER_TOWNS {
        g.state.town = town;
        super::prices::generate_prices(&mut g.state, &mut g.rng);
        for i in 0..NUM_GOODS {
            let ask = g.buy_price(i);
            let bid = g.sell_price(i);
            assert!(ask >= bid, "town {town} good {i}: ask {ask} < bid {bid}");
            assert!(bid > 0.0, "town {town} good {i}: non-positive bid {bid}");
        }
    }
}

#[test]
fn reputation_sets_the_interest_rate() {
    let mut g = started(1);
    // Neutral standing pays the 5% base; a good name borrows cheaper, a bad one
    // dearer, and the rate is clamped to the [-50, 50] reputation band's ends.
    g.state.reputation = 0.0;
    assert!((g.interest_rate() - 0.05).abs() < 1e-9, "neutral is 5%");
    g.state.reputation = 50.0;
    assert!((g.interest_rate() - 0.025).abs() < 1e-9, "top standing is 2.5%");
    g.state.reputation = -50.0;
    assert!((g.interest_rate() - 0.075).abs() < 1e-9, "bottom standing is 7.5%");
}

#[test]
fn reputation_scales_the_lender_credit_cap() {
    use super::state::CINCINNATI;
    // The Cincinnati/Memphis offer is double cash at a neutral name, scaled ±50%
    // by reputation — so the well-regarded leave with more borrowing room. Drive a
    // real arrival so the credit logic in `arrive_at_town` is what's under test.
    let cap = |rep: f64| {
        let mut g = started(1);
        g.build(BoatKind::Flatboat, 2).unwrap();
        // High cash so the offer (≈ cash × multiple) clears the base floor and the
        // reputation scaling is what separates the three runs.
        g.state.cash = 500.0;
        g.state.reputation = rep;
        g.state.town = CINCINNATI - 1; // Maysville: one plain leg above Cincinnati
        g.mode = Mode::Town;
        g.depart();
        settle(&mut g);
        assert_eq!(g.state.town, CINCINNATI, "reached Cincinnati");
        g.state.credit_cap
    };
    let neutral = cap(0.0);
    assert!(cap(50.0) > neutral, "a good name extends more credit than neutral");
    assert!(cap(-50.0) < neutral, "a bad name extends less credit than neutral");
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

