//! Engine tests: spec-anchored unit checks, a multi-seed scripted smoke test
//! that plays whole journeys through the public API, and a behavioral check
//! that good play beats bad play.

use super::interaction::{Response, ShotPurpose, Tactic};
use super::state::{EatLevel, GameOverCause, Mode};
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

/// Stand a game up mid-encounter with hostile riders, ready for a tactic.
fn hostile_riders(seed: u64) -> Game {
    let mut g = fresh(seed);
    g.outfit(240.0, 200.0, 100.0, 50.0, 60.0).unwrap();
    g.riders = Some(RiderEncounter {
        looks_hostile: true,
        hostile: true,
    });
    g.mode = Mode::Riders;
    g
}

#[test]
fn running_from_hostile_riders_opens_the_escape_minigame() {
    let mut g = hostile_riders(3);
    g.resolve_tactic(Tactic::Run);
    // Run no longer resolves instantly — it hands off to the route-memory game.
    assert_eq!(g.mode, Mode::Flee);
    assert!(g.riders.is_some(), "encounter is still pending its outcome");
}

#[test]
fn a_clean_escape_loses_the_riders_and_gains_ground() {
    let mut g = hostile_riders(4);
    g.resolve_tactic(Tactic::Run);
    let (misc0, oxen0, miles0) = (g.state.misc, g.state.oxen, g.state.miles);
    g.resolve_flee(true, 1.0);
    // Lost them: encounter cleared, ground gained, and a clean line scatters
    // only the minimum, never dropping into a gunfight.
    assert!(g.riders.is_none());
    assert!(g.state.miles > miles0, "a clean getaway gains ground");
    assert_eq!(g.state.misc, misc0 - 5.0); // drop == 0 at full accuracy
    assert_eq!(g.state.oxen, oxen0 - 10.0);
    assert_ne!(g.mode, Mode::Flee);
    assert_ne!(g.mode, Mode::Shoot);
}

#[test]
fn a_botched_escape_drops_you_into_the_gunfight() {
    let mut g = hostile_riders(5);
    g.resolve_tactic(Tactic::Run);
    g.resolve_flee(false, 0.0);
    // Run down: the riders catch you and it's the marksmanship game now.
    assert_eq!(g.mode, Mode::Shoot);
    assert_eq!(g.shot, Some(ShotPurpose::Riders { circle: false }));
}

#[test]
fn resolve_flee_ignores_stale_double_taps() {
    let mut g = hostile_riders(6);
    g.resolve_tactic(Tactic::Run);
    g.resolve_flee(true, 1.0); // first call lands the escape
    let snapshot = g.clone();
    g.resolve_flee(false, 0.0); // stale: mode is no longer Flee
    assert_eq!(g, snapshot, "a second resolve must be a no-op");
}

/// Stand a game up at the climb screen with the passes already behind it, so a
/// `resolve_climb` is a pure miles tally (no blizzard gamble to muddy it).
fn mid_climb(seed: u64, miles: f64) -> Game {
    let mut g = fresh(seed);
    g.outfit(240.0, 200.0, 100.0, 50.0, 60.0).unwrap();
    g.state.miles = miles;
    g.state.cleared_south_pass = true;
    g.state.cleared_blue_mountains = true;
    g.resume = Resume::NextTurn; // the Mountains leg's resume point
    g.mode = Mode::Climb;
    g
}

#[test]
fn rugged_mountains_sometimes_launch_the_climb() {
    // The ~30% rugged roll should put up the climb minigame for some — but not
    // all — fortnights past mile 950, and a launched climb pauses the leg.
    let mut launched = 0;
    for seed in 0..200u64 {
        let mut g = fresh(seed);
        g.outfit(240.0, 200.0, 100.0, 50.0, 60.0).unwrap();
        g.state.miles = 1000.0;
        g.resume = Resume::NextTurn;
        g.leg = None;
        let flow = g.do_mountains();
        if g.mode == Mode::Climb {
            assert_eq!(flow, Flow::Pause, "a launched climb pauses the leg");
            launched += 1;
        }
    }
    assert!(launched > 0, "the rugged climb never fired in 200 seeds");
    assert!(launched < 200, "the rugged climb fired every single time");
}

#[test]
fn a_clean_climb_costs_little_ground() {
    let mut g = mid_climb(11, 1200.0);
    g.resolve_climb(true, 1.0);
    assert_eq!(g.state.miles, 1185.0); // -15 at full accuracy, passes already clear
    assert_ne!(g.mode, Mode::Climb);
}

#[test]
fn a_rough_climb_loses_the_full_stretch() {
    let mut g = mid_climb(12, 1200.0);
    g.resolve_climb(false, 0.0);
    assert_eq!(g.state.miles, 1105.0); // -95 at zero accuracy
    assert_ne!(g.mode, Mode::Climb);
}

#[test]
fn resolving_a_climb_falls_through_to_the_passes() {
    let mut g = fresh(7);
    g.outfit(240.0, 200.0, 100.0, 50.0, 60.0).unwrap();
    g.state.miles = 1000.0;
    g.resume = Resume::NextTurn;
    g.mode = Mode::Climb;
    assert!(!g.state.cleared_south_pass, "South Pass starts pending");
    g.resolve_climb(true, 1.0);
    // The crossing flowed on into the one-time South Pass check.
    assert!(g.state.cleared_south_pass);
}

#[test]
fn resolve_climb_ignores_stale_double_taps() {
    let mut g = mid_climb(13, 1200.0);
    g.resolve_climb(true, 1.0); // first call lands the crossing
    let snapshot = g.clone();
    g.resolve_climb(false, 0.0); // stale: mode is no longer Climb
    assert_eq!(g, snapshot, "a second resolve must be a no-op");
}

/// Stand a game up at the fog screen, mid-trail, ready for a `resolve_fog`.
fn mid_fog(seed: u64, miles: f64) -> Game {
    let mut g = fresh(seed);
    g.outfit(240.0, 200.0, 100.0, 50.0, 60.0).unwrap();
    g.state.miles = miles;
    g.resume = Resume::Trail; // drains somewhere harmless after resolving
    g.leg = None;
    g.mode = Mode::Fog;
    g
}

#[test]
fn heavy_fog_sometimes_launches_the_navigation_game() {
    // Event 10 (~5%) should put up the fog minigame for some seeds, and a
    // launched fog pauses the event leg. Mileage held below the mountains so
    // only the random-event roll is in play.
    let mut launched = 0;
    for seed in 0..400u64 {
        let mut g = fresh(seed);
        g.outfit(240.0, 200.0, 100.0, 50.0, 60.0).unwrap();
        g.state.miles = 300.0;
        g.resume = Resume::Leg;
        g.leg = Some(Leg::Mountains);
        let flow = g.do_event();
        if g.mode == Mode::Fog {
            assert_eq!(flow, Flow::Pause, "a launched fog pauses the leg");
            launched += 1;
        }
    }
    assert!(launched > 0, "heavy fog never fired in 400 seeds");
}

#[test]
fn keeping_your_bearings_in_fog_costs_no_time() {
    let mut g = mid_fog(21, 600.0);
    g.resolve_fog(true, 1.0);
    assert_eq!(g.state.miles, 600.0); // held the trail → no time lost
    assert_ne!(g.mode, Mode::Fog);
}

#[test]
fn losing_your_way_in_fog_costs_time_by_how_far_you_drifted() {
    let mut total = mid_fog(22, 600.0);
    total.resolve_fog(false, 0.0); // total whiteout from the start
    assert_eq!(total.state.miles, 585.0); // -(5 + 10) at zero accuracy

    let mut nearly = mid_fog(22, 600.0);
    nearly.resolve_fog(false, 0.8); // drifted only near the end
    // A near-miss wanders less than a total loss, but still costs something.
    assert!(nearly.state.miles > 585.0 && nearly.state.miles < 600.0);
}

#[test]
fn resolve_fog_ignores_stale_double_taps() {
    let mut g = mid_fog(23, 600.0);
    g.resolve_fog(false, 0.0); // first call lands the wander
    let snapshot = g.clone();
    g.resolve_fog(true, 1.0); // stale: mode is no longer Fog
    assert_eq!(g, snapshot, "a second resolve must be a no-op");
}

/// Stand a game up outfitted and parked on the event leg, ready to roll events.
fn ready_for_event(seed: u64, miles: f64) -> Game {
    let mut g = fresh(seed);
    g.outfit(240.0, 200.0, 100.0, 50.0, 60.0).unwrap();
    g.state.miles = miles;
    g.resume = Resume::Leg;
    g.leg = Some(Leg::Mountains);
    g
}

#[test]
fn a_broken_arm_launches_the_splint_game() {
    // Event 3 should put up the splint minigame for some seeds, and a launched
    // splint pauses the event leg. Below the mountains so only the event roll
    // is in play.
    let mut launched = 0;
    for seed in 0..400u64 {
        let mut g = ready_for_event(seed, 300.0);
        let flow = g.do_event();
        if g.mode == Mode::Splint {
            assert_eq!(flow, Flow::Pause, "a launched splint pauses the leg");
            launched += 1;
        }
    }
    assert!(launched > 0, "the broken arm never fired in 400 seeds");
}

#[test]
fn a_clean_splint_costs_less_than_a_botched_one() {
    let mut clean = ready_for_event(7, 300.0);
    clean.mode = Mode::Splint;
    let (m0, x0) = (clean.state.miles, clean.state.misc);
    clean.resolve_splint(true, 1.0);
    let clean_loss = (m0 - clean.state.miles) + (x0 - clean.state.misc);
    assert_ne!(clean.mode, Mode::Splint);

    let mut botched = ready_for_event(7, 300.0);
    botched.mode = Mode::Splint;
    let (m1, x1) = (botched.state.miles, botched.state.misc);
    botched.resolve_splint(false, 0.0);
    let botched_loss = (m1 - botched.state.miles) + (x1 - botched.state.misc);

    assert!(
        botched_loss > clean_loss,
        "fumbling the set should cost more time and supplies"
    );
}

#[test]
fn resolve_splint_ignores_stale_double_taps() {
    let mut g = ready_for_event(7, 300.0);
    g.mode = Mode::Splint;
    g.resolve_splint(false, 0.0);
    let snapshot = g.clone();
    g.resolve_splint(true, 1.0); // stale: mode is no longer Splint
    assert_eq!(g, snapshot, "a second resolve must be a no-op");
}

#[test]
fn illness_launches_the_dosing_game_and_a_shaky_pour_wastes_supplies() {
    // Drive the eat-then-sicken roll until event 15 makes the party ill; a
    // launched illness pauses for the dose and stashes a severity.
    let mut launched = 0;
    for seed in 0..400u64 {
        let mut g = ready_for_event(seed, 300.0);
        g.state.eat_level = EatLevel::Poorly; // poor eating always sickens on event 15
        let flow = g.do_event();
        if g.mode == Mode::Dose {
            assert_eq!(flow, Flow::Pause, "a launched illness pauses the leg");
            assert!(g.illness_task().is_some(), "a severity must be stashed");

            // Same illness, two pours: the steady one keeps more medical supplies.
            let mut steady = g.clone();
            steady.resolve_dose(true, 1.0);
            let mut shaky = g.clone();
            shaky.resolve_dose(false, 0.0);
            assert!(
                shaky.state.misc < steady.state.misc,
                "spilling the dose should burn extra supplies"
            );
            assert!(steady.illness_task().is_none(), "the dose clears the illness");
            launched += 1;
        }
    }
    assert!(launched > 0, "illness never fired in 400 seeds");
}

/// Stand a game up at the steady-hand (ice crossing) screen, ready to resolve.
fn mid_steady(seed: u64, miles: f64) -> Game {
    let mut g = ready_for_event(seed, miles);
    g.pending_task = Some(MiniTask::Ice);
    g.mode = Mode::Steady;
    g
}

/// Stand a game up at the order-memory screen for a given task, ready to resolve.
fn mid_sequence(seed: u64, miles: f64, task: SequenceTask) -> Game {
    let mut g = ready_for_event(seed, miles);
    g.pending_task = Some(MiniTask::Sequence(task));
    g.mode = Mode::Sequence;
    g
}

#[test]
fn the_frozen_cumberland_launches_the_steady_game() {
    // The Christmas crossing is scripted: once the party reaches the river mile
    // with the earlier passes behind it, the steady-hand ice trace fires exactly
    // once and pauses the leg.
    let mut g = fresh(7);
    g.outfit(240.0, 200.0, 100.0, 50.0, 60.0).unwrap();
    g.state.miles = super::scenario_data::scenario().trail.cumberland_river_at;
    g.state.cleared_south_pass = true;
    g.state.cleared_blue_mountains = true;
    assert!(!g.state.cleared_cumberland_river, "the river starts uncrossed");

    let flow = g.do_mountain_passes();
    assert_eq!(flow, Flow::Pause, "the ice crossing pauses the leg");
    assert_eq!(g.mode, Mode::Steady);
    assert!(g.is_ice_crossing());
    assert!(g.state.cleared_cumberland_river, "the crossing is marked started");

    // It only fires once: a second pass-check (river already crossed) doesn't
    // re-launch it.
    let mut h = fresh(8);
    h.outfit(240.0, 200.0, 100.0, 50.0, 60.0).unwrap();
    h.state.miles = super::scenario_data::scenario().trail.cumberland_river_at;
    h.state.cleared_south_pass = true;
    h.state.cleared_blue_mountains = true;
    h.state.cleared_cumberland_river = true;
    assert_eq!(h.do_mountain_passes(), Flow::Continue);
    assert_ne!(h.mode, Mode::Steady);
}

#[test]
fn a_botched_ice_crossing_can_break_the_ice() {
    // A badly shaky run drops the party through the ice — a fatal ending.
    let mut g = mid_steady(7, super::scenario_data::scenario().trail.cumberland_river_at);
    g.resolve_steady(false, 0.0);
    let end = g.outcome.expect("a broken-ice crossing should end the journey");
    assert!(!end.won, "going through the ice is a loss");
    assert_eq!(end.cause_kind, GameOverCause::IceBroke);
}

#[test]
fn the_ordered_procedure_catastrophes_launch_the_sequence_game() {
    // Events 1 (wagon wheel), 2 (ox leg), and 11 (snakebite) each pause into the
    // order-memory game; over enough seeds all three tasks should turn up. Below
    // the mountains so only the event roll is in play.
    let (mut saw_wheel, mut saw_ox, mut saw_snake) = (false, false, false);
    for seed in 0..1500u64 {
        let mut g = ready_for_event(seed, 300.0);
        let flow = g.do_event();
        if g.mode == Mode::Sequence {
            assert_eq!(flow, Flow::Pause, "a launched sequence pauses the leg");
            match g.sequence_task().expect("a task must be stashed") {
                SequenceTask::Wheel => saw_wheel = true,
                SequenceTask::OxLeg => saw_ox = true,
                SequenceTask::Frostbite => saw_snake = true,
            }
        }
    }
    assert!(
        saw_wheel && saw_ox && saw_snake,
        "all three ordered-procedure catastrophes should fire across the seed sweep"
    );
}

#[test]
fn a_steady_hand_costs_less_than_a_shaky_one() {
    // Both runs survive (a moderate shaky run, not a fatal one), so the losses
    // are comparable: a steadier line over the ice keeps more stock and supplies.
    let mut steady = mid_steady(7, 300.0);
    let (m0, f0, o0) = (steady.state.miles, steady.state.food, steady.state.oxen);
    steady.resolve_steady(true, 1.0);
    let steady_loss =
        (m0 - steady.state.miles) + (f0 - steady.state.food) + (o0 - steady.state.oxen);
    assert_ne!(steady.mode, Mode::Steady);

    let mut shaky = mid_steady(7, 300.0);
    let (m1, f1, o1) = (shaky.state.miles, shaky.state.food, shaky.state.oxen);
    shaky.resolve_steady(false, 0.5);
    let shaky_loss =
        (m1 - shaky.state.miles) + (f1 - shaky.state.food) + (o1 - shaky.state.oxen);

    assert!(
        shaky_loss > steady_loss,
        "letting the herd slip on the ice should cost more time and supplies"
    );
}

#[test]
fn a_broken_ice_crossing_shows_the_splinter_line_before_game_over() {
    // A badly shaky crossing breaks the ice; the narration must be drained (shown
    // in Interaction mode) before the GameOver screen, not orphaned in the queue.
    let mut g = mid_steady(7, super::scenario_data::scenario().trail.cumberland_river_at);
    g.resolve_steady(false, 0.0); // drift > 0.6 → the ice gives way
    assert!(
        g.outcome
            .as_ref()
            .is_some_and(|e| e.cause_kind == GameOverCause::IceBroke),
        "the party goes into the river"
    );
    assert_eq!(g.mode, Mode::Interaction, "the splinter line shows first");
    assert!(!g.pending.is_empty(), "the narration is still queued to show");
    g.resolve(Response::Acknowledge);
    assert_eq!(g.mode, Mode::GameOver);
}

#[test]
fn resolve_steady_ignores_stale_double_taps() {
    let mut g = mid_steady(7, 300.0);
    g.resolve_steady(false, 0.5); // first call lands the crossing (survives)
    let snapshot = g.clone();
    g.resolve_steady(true, 1.0); // stale: mode is no longer Steady
    assert_eq!(g, snapshot, "a second resolve must be a no-op");
}

#[test]
fn a_clean_order_costs_less_than_a_botched_one() {
    // Re-seating the wheel in the right order barely costs a step; fumbling it
    // from the first slip loses the most ground and spare parts.
    let mut clean = mid_sequence(7, 300.0, SequenceTask::Wheel);
    let (m0, x0) = (clean.state.miles, clean.state.misc);
    clean.resolve_sequence(4, 4, true);
    let clean_loss = (m0 - clean.state.miles) + (x0 - clean.state.misc);
    assert_ne!(clean.mode, Mode::Sequence);

    let mut botched = mid_sequence(7, 300.0, SequenceTask::Wheel);
    let (m1, x1) = (botched.state.miles, botched.state.misc);
    botched.resolve_sequence(0, 4, false);
    let botched_loss = (m1 - botched.state.miles) + (x1 - botched.state.misc);

    assert!(
        botched_loss > clean_loss,
        "fumbling the order should cost more time and supplies"
    );
}

#[test]
fn a_clean_order_saves_the_medicine_that_keeps_you_alive() {
    // With medicine all but gone, a flawless first-aid order survives the frostbite
    // on what's left; a botched one wastes the last of it and it turns fatal.
    let mut clean = mid_sequence(7, 300.0, SequenceTask::Frostbite);
    clean.state.misc = 5.0;
    clean.resolve_sequence(5, 5, true);
    assert!(
        clean.outcome.is_none(),
        "a flawless order should survive on the medicine left"
    );

    let mut botched = mid_sequence(7, 300.0, SequenceTask::Frostbite);
    botched.state.misc = 5.0;
    botched.resolve_sequence(0, 5, false);
    let end = botched
        .outcome
        .expect("wasting the last medicine should be fatal");
    assert!(!end.won, "frostbite death is a loss");
}

#[test]
fn resolve_sequence_ignores_stale_double_taps() {
    let mut g = mid_sequence(7, 300.0, SequenceTask::OxLeg);
    g.resolve_sequence(0, 4, false); // first call dresses the leg
    let snapshot = g.clone();
    g.resolve_sequence(4, 4, true); // stale: mode is no longer Sequence
    assert_eq!(g, snapshot, "a second resolve must be a no-op");
}

/// Stand a game up at the bucket-brigade screen for a given task, ready to resolve.
fn mid_brigade(seed: u64, miles: f64, task: BrigadeTask) -> Game {
    let mut g = ready_for_event(seed, miles);
    g.pending_task = Some(MiniTask::Brigade(task));
    g.mode = Mode::Brigade;
    g
}

#[test]
fn fire_and_rains_launch_the_brigade_game() {
    // Events 9 (fire) and 7 (heavy rains, below the mountains) each pause into
    // the bucket-brigade triage; over enough seeds both should turn up. Held
    // below the mountains so the rains stay rains (not cold weather).
    let (mut saw_fire, mut saw_rains) = (false, false);
    for seed in 0..1500u64 {
        let mut g = ready_for_event(seed, 300.0);
        let flow = g.do_event();
        if g.mode == Mode::Brigade {
            assert_eq!(flow, Flow::Pause, "a launched brigade pauses the leg");
            match g.brigade_task().expect("a task must be stashed") {
                BrigadeTask::Fire => saw_fire = true,
                BrigadeTask::Rains => saw_rains = true,
                BrigadeTask::Blizzard => panic!("no blizzard below the mountains"),
            }
        }
    }
    assert!(
        saw_fire && saw_rains,
        "both fire and heavy rains should fire across the seed sweep"
    );
}

#[test]
fn containing_a_blaze_costs_less_than_letting_it_run() {
    let mut contained = mid_brigade(7, 300.0, BrigadeTask::Fire);
    let (f0, b0, m0) = (
        contained.state.food,
        contained.state.bullets,
        contained.state.misc,
    );
    contained.resolve_brigade(true, 0, 25);
    let kept_loss = (f0 - contained.state.food)
        + (b0 - contained.state.bullets)
        + (m0 - contained.state.misc);
    assert_ne!(contained.mode, Mode::Brigade);

    let mut lost = mid_brigade(7, 300.0, BrigadeTask::Fire);
    let (f1, b1, m1) = (lost.state.food, lost.state.bullets, lost.state.misc);
    lost.resolve_brigade(false, 25, 25);
    let run_loss = (f1 - lost.state.food) + (b1 - lost.state.bullets) + (m1 - lost.state.misc);

    assert!(
        run_loss > kept_loss,
        "letting the fire spread should destroy more than stamping it out"
    );
}

#[test]
fn partial_containment_grades_between_floor_and_full() {
    // The engine now owns the leaked/capacity -> severity mapping (the screen used
    // to do it), so it's unit-testable here: a half-leaked grid must cost strictly
    // more than full containment and strictly less than a total loss.
    let loss = |leaked: usize| {
        let mut g = mid_brigade(7, 300.0, BrigadeTask::Fire);
        let (f0, b0) = (g.state.food, g.state.bullets);
        g.resolve_brigade(leaked == 0, leaked, 25);
        (f0 - g.state.food) + (b0 - g.state.bullets)
    };
    let (floor, half, full) = (loss(0), loss(12), loss(25));
    assert!(
        floor < half && half < full,
        "loss should scale with how much leaked: {floor} < {half} < {full}"
    );
}

#[test]
fn an_underclothed_blizzard_brings_on_sickness() {
    // The blizzard's cold-weather check still bites: with no clothing the pass
    // drops you straight into the dosing game.
    let mut g = mid_brigade(7, 1200.0, BrigadeTask::Blizzard);
    g.state.cleared_south_pass = true; // a clean fall-through, no second gamble
    g.state.cleared_blue_mountains = true;
    g.resume = Resume::NextTurn;
    g.state.clothing = 0.0;
    g.resolve_brigade(false, 25, 25);
    assert_eq!(g.mode, Mode::Dose, "freezing in the pass should bring on illness");
}

#[test]
fn a_well_clothed_party_rides_out_the_blizzard() {
    // Enough clothing skips the illness, and the resolve falls through the passes
    // and carries on — landing anywhere but back on the brigade or in the doctor.
    let mut g = mid_brigade(7, 1200.0, BrigadeTask::Blizzard);
    g.state.cleared_south_pass = true;
    g.state.cleared_blue_mountains = true;
    g.resume = Resume::NextTurn;
    g.state.clothing = 100.0;
    g.resolve_brigade(true, 0, 25);
    assert_ne!(g.mode, Mode::Brigade);
    assert_ne!(g.mode, Mode::Dose);
    assert!(g.brigade_task().is_none(), "the resolve clears the task");
}

#[test]
fn resolve_brigade_ignores_stale_double_taps() {
    let mut g = mid_brigade(7, 300.0, BrigadeTask::Fire);
    g.resolve_brigade(false, 25, 25); // first call tallies the blaze
    let snapshot = g.clone();
    g.resolve_brigade(true, 0, 25); // stale: mode is no longer Brigade
    assert_eq!(g, snapshot, "a second resolve must be a no-op");
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
            // Good play threads the route clean; bad play fouls it immediately.
            Mode::Flee => g.resolve_flee(good, if good { 1.0 } else { 0.0 }),
            Mode::Climb => g.resolve_climb(good, if good { 1.0 } else { 0.0 }),
            Mode::Fog => g.resolve_fog(good, if good { 1.0 } else { 0.0 }),
            // Good play sets the bone / pours the dose dead-center; bad play fumbles.
            Mode::Splint => g.resolve_splint(good, if good { 1.0 } else { 0.0 }),
            Mode::Dose => g.resolve_dose(good, if good { 1.0 } else { 0.0 }),
            // Good play holds the trace dead steady; bad play lets it wander off.
            Mode::Steady => g.resolve_steady(good, if good { 1.0 } else { 0.0 }),
            // Good play reproduces the whole order; bad play slips on the first step.
            Mode::Sequence => g.resolve_sequence(if good { 4 } else { 0 }, 4, good),
            // Good play beats the spread back to zero; bad play lets it run wild.
            Mode::Brigade => g.resolve_brigade(good, if good { 0 } else { 25 }, 25),
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
    let trail = &super::scenario_data::scenario().trail;
    for seed in 0..40u64 {
        let g = play(seed, true);
        let end = g.outcome.expect("game ended without an outcome");
        // The journey can't run past the winter deadline.
        assert!(g.state.turn <= trail.max_turns, "seed {seed} ran too long");
        // The final report never shows negative supplies.
        assert!(end.food >= 0 && end.bullets >= 0 && end.clothing >= 0);
        assert!(end.misc >= 0 && end.cash >= 0);
        assert!(end.miles >= 0 && end.miles <= trail.total_miles as i64);
        assert!(!end.rank.is_empty());
    }
}

#[test]
fn good_play_outlasts_bad_play() {
    let seeds = 0..60u64;
    let mut good_total = 0i64;
    let mut bad_total = 0i64;
    let mut good_wins = 0;
    let total_miles = super::scenario_data::scenario().trail.total_miles;
    for seed in seeds {
        let good = play(seed, true);
        good_total += good.state.miles.min(total_miles) as i64;
        if good.outcome.is_some_and(|e| e.won) {
            good_wins += 1;
        }
        bad_total += play(seed, false).state.miles.min(total_miles) as i64;
    }
    assert!(
        good_total > bad_total,
        "good {good_total} should outdistance bad {bad_total}"
    );
    // The trail is winnable with good play on at least some seeds.
    assert!(good_wins >= 1, "good play never won across 60 seeds");
}

// ----- Cover-art key resolution -----

#[test]
fn cover_keys_supersede_then_fall_back() {
    use crate::ui::components::cover::cover_keys;

    // The trail hub keys on the current checkpoint, then the generic key.
    let mut g = fresh(1);
    g.mode = Mode::Trail;
    g.state.miles = 0.0; // Fort Patrick Henry
    assert_eq!(
        cover_keys(&g),
        vec!["trail-fort-patrick-henry".to_string(), "trail".to_string()]
    );
    g.state.miles = 600.0; // Martin's Station country
    assert_eq!(
        cover_keys(&g),
        vec!["trail-martins-station".to_string(), "trail".to_string()]
    );

    // A narrative minigame key supersedes the general Sequence key.
    g.mode = Mode::Sequence;
    g.pending_task = Some(MiniTask::Sequence(SequenceTask::Frostbite));
    assert_eq!(
        cover_keys(&g),
        vec!["sequence-frostbite".to_string(), "sequence".to_string()]
    );

    // A tagged trail incident keys on its slug, then the generic interaction key.
    let mut g = fresh(1);
    g.message_keyed("A pack animal wanders off.", "livestock-strays");
    g.mode = Mode::Interaction;
    assert_eq!(
        cover_keys(&g),
        vec![
            "interaction-livestock-strays".to_string(),
            "interaction".to_string()
        ]
    );

    // An ending keys on its cause, then the generic game-over key.
    let mut g = fresh(1);
    g.die(GameOverCause::Frostbite);
    assert_eq!(g.mode, Mode::GameOver);
    assert_eq!(
        cover_keys(&g),
        vec!["game-over-frostbite".to_string(), "game-over".to_string()]
    );
}

// ===== Golden-trace harness =====
//
// Behavior-preservation oracle for the data-driven refactor. A fully scripted,
// deterministic run across seven seeds, snapshotting the whole world state after
// every engine transition into one long string, then hashing it. The minigame /
// interaction feed is deterministic but *varied* (it cycles through clean runs,
// botched runs, and the occasional catastrophe) so both the winning and losing
// branches of every hazard are exercised. While the refactor preserves behavior
// the hash holds; the moment a coefficient, an RNG draw order, or a branch drifts
// the hash changes and the gate trips. Run `cargo test -p fort-nash` after every
// stage. To eyeball a drift, `cargo test -p fort-nash print_golden_trace --
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
        "{:?}|c{:?}|fd{:?}|bu{:?}|cl{:?}|mc{:?}|ox{:?}|mi{:?}|m0{:?}|tn{}|mk{}|il{}|in{}|sp{}|bm{}|cr{}|eat{:?}\n",
        g.mode,
        s.cash,
        s.food,
        s.bullets,
        s.clothing,
        s.misc,
        s.oxen,
        s.miles,
        s.miles_at_turn_start,
        s.turn,
        s.marksman,
        s.ill,
        s.injured,
        s.cleared_south_pass,
        s.cleared_blue_mountains,
        s.cleared_cumberland_river,
        s.eat_level,
    );
    // Also snapshot the resume/leg bookkeeping, the pending encounter/shot/task,
    // and the narration queue (message text + cover keys). Every prose string and
    // cover slug — the bulk of the data-driven port — folds into the hash, closing
    // the gap where the numeric snapshot alone couldn't catch a dropped cover key
    // or a reworded message.
    let _ = write!(
        log,
        "  leg{:?} res{:?} rid{:?} shot{:?} task{:?}\n  q{:?}\n",
        g.leg, g.resume, g.riders, g.shot, g.pending_task, g.pending,
    );
}

/// Resolve whatever minigame / interaction sits at the head with a
/// varied-but-deterministic answer, snapshotting after each step, until we land
/// on a hub (Trail / Eat / Fort / Riders) or an ending.
fn settle_feed(g: &mut Game, feed: &mut Feed, log: &mut String) {
    let mut guard = 0;
    loop {
        match g.mode {
            Mode::Interaction => g.resolve(Response::Acknowledge),
            Mode::Shoot => {
                let (secs, correct) = match feed.next() % 4 {
                    0 => (0.4, true),  // a dead-eye draw
                    1 => (1.4, true),  // a hit, but slow
                    2 => (3.5, false), // a flubbed word
                    _ => (0.8, true),
                };
                g.resolve_shot(secs, correct);
            }
            Mode::Hunt => {
                let (hit, shots) = match feed.next() % 3 {
                    0 => (true, 1),  // a clean one-shot kill
                    1 => (false, 6), // emptied the bag, missed
                    _ => (true, 3),
                };
                g.resolve_hunt(hit, shots);
            }
            Mode::Flee => {
                let (cleared, acc) = match feed.next() % 3 {
                    0 => (true, 0.95),
                    1 => (false, 0.15), // run down into the gunfight
                    _ => (true, 0.7),
                };
                g.resolve_flee(cleared, acc);
            }
            Mode::Climb => {
                let (cleared, acc) = match feed.next() % 3 {
                    0 => (true, 0.95),
                    1 => (false, 0.2),
                    _ => (true, 0.6),
                };
                g.resolve_climb(cleared, acc);
            }
            Mode::Fog => {
                let (cleared, acc) = match feed.next() % 3 {
                    0 => (true, 0.95),
                    1 => (false, 0.2),
                    _ => (false, 0.7),
                };
                g.resolve_fog(cleared, acc);
            }
            Mode::Splint => {
                let (clean, acc) = match feed.next() % 3 {
                    0 => (true, 0.95),
                    1 => (false, 0.1),
                    _ => (true, 0.6),
                };
                g.resolve_splint(clean, acc);
            }
            Mode::Dose => {
                let (on_target, acc) = match feed.next() % 4 {
                    0 => (true, 0.95),
                    1 => (false, 0.5),
                    2 => (false, 0.05), // a badly spilled dose
                    _ => (true, 0.6),
                };
                g.resolve_dose(on_target, acc);
            }
            Mode::Steady => {
                let (steady, acc) = match feed.next() % 5 {
                    0 => (true, 0.95),
                    1 => (false, 0.55),
                    2 => (true, 0.80),
                    3 => (false, 0.30),
                    _ => (false, 0.10), // drift > 0.6 → the ice gives way
                };
                g.resolve_steady(steady, acc);
            }
            Mode::Sequence => {
                let (prefix, len, perfect) = match feed.next() % 3 {
                    0 => (4, 4, true),
                    1 => (0, 4, false),
                    _ => (2, 4, false),
                };
                g.resolve_sequence(prefix, len, perfect);
            }
            Mode::Brigade => {
                let (contained, leaked, cap) = match feed.next() % 3 {
                    0 => (true, 0, 25),
                    1 => (false, 25, 25),
                    _ => (false, 12, 25),
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
    let mut feed = Feed {
        n: seed.wrapping_mul(2_654_435_761),
    };
    let mut g = Game::new(seed);
    // Vary marksmanship by seed to exercise the gunfight handicap; a fixed,
    // well-stocked outfit otherwise (total $700, enough powder to hunt).
    g.begin("Golden Party".into(), (seed % 5) as u8 + 1);
    let _ = write!(log, "=== seed {seed} ===\n");
    g.outfit(240.0, 300.0, 80.0, 40.0, 40.0).unwrap();
    snap(&g, log);

    let mut guard = 0;
    loop {
        settle_feed(&mut g, &mut feed, log);
        match g.mode {
            Mode::Trail => {
                // Survive deep enough to reach the passes and the Christmas ice
                // crossing: hunt when the larder runs low, stop at a station when
                // one's offered, otherwise press on.
                if g.state.food < 90.0 && g.state.bullets > 39.0 {
                    let _ = g.choose_hunt();
                } else if g.state.fort_available() && feed.next() % 2 == 0 {
                    g.choose_fort();
                } else {
                    g.choose_continue();
                }
            }
            Mode::Fort => {
                if feed.next() % 2 == 0 {
                    g.buy_at_fort(40.0, 10.0, 10.0, 10.0);
                } else {
                    g.leave_fort();
                }
            }
            Mode::Eat => {
                // Eat as well as the larder allows — well-fed parties travel
                // farther, so the trace reaches the mountains, the ice, and the
                // French Lick. Minigame variety (above) still drives the branches.
                let lvl = if g.state.food >= EatLevel::Well.food_cost() {
                    EatLevel::Well
                } else if g.state.food >= EatLevel::Moderately.food_cost() {
                    EatLevel::Moderately
                } else {
                    EatLevel::Poorly
                };
                g.choose_eat(lvl);
            }
            Mode::Riders => {
                let tactic = match feed.next() % 4 {
                    0 => Tactic::Run,
                    1 => Tactic::Attack,
                    2 => Tactic::Continue,
                    _ => Tactic::CircleWagons,
                };
                g.resolve_tactic(tactic);
            }
            Mode::GameOver => break,
            other => panic!("seed {seed}: unexpected mode {other:?}"),
        }
        snap(&g, log);
        guard += 1;
        assert!(guard < 2000, "seed {seed}: run never ended (mode {:?})", g.mode);
    }
    let _ = write!(log, "outcome {:?}\n", g.outcome);
}

/// The full deterministic trace across every seed.
fn golden_trace() -> String {
    let mut log = String::new();
    for seed in [
        11u64, 22, 33, 44, 55, 66, 77, 88, 99, 111, 222, 333, 444, 555,
    ] {
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
    // Baseline captured before the data-driven refactor. Behavior must not drift;
    // if this trips, run `print_golden_trace` (below) before and after to diff.
    const EXPECTED: u64 = 0xc26e_ed8b_ea10_a5f7;
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

// ===== Scenario data-shape consistency =====
//
// While behaviour is locked by the golden trace, these guard the *shape* of the
// embedded RON: that it parses, lines up with the engine's structural constants,
// and that every id / cause key it cross-references actually resolves.

#[test]
fn scenario_parses() {
    let _ = super::scenario_data::scenario();
}

#[test]
fn scenario_is_self_consistent() {
    use trail_kit::fortnash::{Effect, HazardArm, MinigameParams};
    let sc = super::scenario_data::scenario();

    // Checkpoints run from the start to the trail's end, strictly ascending.
    assert_eq!(sc.checkpoints.first().unwrap().mile, 0.0);
    assert_eq!(
        sc.checkpoints.last().unwrap().mile,
        sc.trail.total_miles
    );
    for w in sc.checkpoints.windows(2) {
        assert!(w[0].mile < w[1].mile, "checkpoints must ascend by mile");
    }

    // One date per week on the trail.
    assert_eq!(sc.dates.len(), sc.trail.max_turns as usize);

    // The checkpoint lookup lands on the right band: at each checkpoint's starting
    // mile, the engine reports that checkpoint's label and cover key.
    use super::state::GameState;
    for cp in &sc.checkpoints {
        let mut s = GameState::new("T".into());
        s.miles = cp.mile;
        assert_eq!(s.terrain(), cp.label, "terrain label at mile {}", cp.mile);
        assert_eq!(s.terrain_key(), cp.key, "terrain key at mile {}", cp.mile);
    }

    // Every ending the engine can reach has scenario text, and only the victory
    // is a win.
    for cause in [
        GameOverCause::Starved,
        GameOverCause::Pneumonia,
        GameOverCause::Frostbite,
        GameOverCause::Winter,
        GameOverCause::CantAffordDoctor,
        GameOverCause::RiderMassacre,
        GameOverCause::Wolves,
        GameOverCause::IceBroke,
        GameOverCause::Victory,
    ] {
        let end = sc
            .ending(cause.key())
            .unwrap_or_else(|| panic!("no ending for cause {}", cause.key()));
        assert_eq!(end.won, cause.won(), "won flag for {}", cause.key());
    }

    // The event table has exactly one more arm than threshold.
    assert_eq!(sc.events.arms.len(), sc.events.thresholds.len() + 1);

    // Collect every outcome id a Minigame event arm fires, and confirm each
    // resolves to a defined outcome. (Branch arms nest, so walk them too.)
    fn collect_minigame_ids<'a>(arm: &'a HazardArm, into: &mut Vec<&'a str>) {
        match arm {
            HazardArm::Minigame { outcome } => into.push(outcome),
            HazardArm::Branch {
                past_mountains,
                before,
            } => {
                collect_minigame_ids(past_mountains, into);
                collect_minigame_ids(before, into);
            }
            _ => {}
        }
    }
    let mut event_ids = Vec::new();
    for arm in &sc.events.arms {
        collect_minigame_ids(arm, &mut event_ids);
    }
    for id in &event_ids {
        assert!(sc.outcome(id).is_some(), "event arm fires unknown outcome {id}");
        assert!(
            sc.minigame_params(id).is_some(),
            "event arm fires minigame {id} with no params"
        );
    }

    // Every string the event table hands to the host resolves to a real handler,
    // so a data-only typo fails this test instead of panicking mid-fortnight: a
    // Minigame arm must be launchable by begin_event_minigame, a Special arm must
    // be known to run_special, and any inline Effects death must name a cause that
    // both has an ending and round-trips through GameOverCause::from_key.
    fn check_event_arm(arm: &HazardArm, sc: &trail_kit::fortnash::Scenario) {
        match arm {
            HazardArm::Minigame { outcome } => assert!(
                super::events::is_event_minigame(outcome),
                "event table fires Minigame({outcome}), which begin_event_minigame can't launch"
            ),
            HazardArm::Special(name) => assert!(
                super::events::is_special_handler(name),
                "event table fires Special({name}), which run_special doesn't handle"
            ),
            HazardArm::Effects(effects) => {
                for e in effects {
                    if let Effect::Die(cause) | Effect::DieIfBroke(cause) = e {
                        assert!(
                            sc.ending(cause).is_some() && GameOverCause::from_key(cause).is_some(),
                            "event Effects arm dies with unresolved cause {cause}"
                        );
                    }
                }
            }
            HazardArm::Branch {
                past_mountains,
                before,
            } => {
                check_event_arm(past_mountains, sc);
                check_event_arm(before, sc);
            }
        }
    }
    for arm in &sc.events.arms {
        check_event_arm(arm, sc);
    }

    // Every minigame id has a matching outcome, and vice-versa, so the host can
    // always pair the screen it launches with the tier it resolves.
    for m in &sc.minigames {
        assert!(
            sc.outcome(&m.id).is_some(),
            "minigame {} has no outcome",
            m.id
        );
    }
    for o in &sc.outcomes {
        assert!(
            sc.minigame_params(&o.id).is_some(),
            "outcome {} has no minigame params",
            o.id
        );
    }

    // Every death an outcome triggers names a real ending, and a death gate is
    // always the last effect in its tier (nothing runs after the journey ends).
    for o in &sc.outcomes {
        for tier in [
            o.success.as_slice(),
            o.partial.as_slice(),
            o.fail.as_slice(),
            o.catastrophe.as_slice(),
        ] {
            for (i, e) in tier.iter().enumerate() {
                if let Effect::Die(cause) | Effect::DieIfBroke(cause) = e {
                    assert!(
                        sc.ending(cause).is_some(),
                        "outcome {} dies with unknown cause {cause}",
                        o.id
                    );
                    // The cause must also round-trip through GameOverCause::from_key,
                    // or kill() panics at runtime when this tier fires.
                    assert!(
                        GameOverCause::from_key(cause).is_some(),
                        "outcome {} dies with cause {cause} that GameOverCause::from_key can't resolve",
                        o.id
                    );
                    assert_eq!(
                        i,
                        tier.len() - 1,
                        "a death gate must be the last effect in outcome {}'s tier",
                        o.id
                    );
                }
            }
        }
    }

    // Each minigame param variant is the kind its task expects.
    let kind_ok = |id: &str, f: fn(&MinigameParams) -> bool| {
        f(sc.minigame_params(id).unwrap())
    };
    assert!(kind_ok("ice", |p| matches!(p, MinigameParams::Steady { .. })));
    for id in ["wheel", "ox-leg", "frostbite"] {
        assert!(kind_ok(id, |p| matches!(p, MinigameParams::Sequence { .. })));
    }
    for id in ["fire", "rains", "blizzard"] {
        assert!(kind_ok(id, |p| matches!(p, MinigameParams::Brigade { .. })));
    }
    for id in ["splint", "dose-mild", "dose-bad", "dose-serious"] {
        assert!(kind_ok(id, |p| matches!(p, MinigameParams::Timing { .. })));
    }
    for id in ["fog", "flee", "climb"] {
        assert!(kind_ok(id, |p| matches!(p, MinigameParams::Crowd { .. })));
    }
}
