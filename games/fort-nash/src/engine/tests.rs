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
    g.state.miles = super::state::CUMBERLAND_RIVER_AT;
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
    h.state.miles = super::state::CUMBERLAND_RIVER_AT;
    h.state.cleared_south_pass = true;
    h.state.cleared_blue_mountains = true;
    h.state.cleared_cumberland_river = true;
    assert_eq!(h.do_mountain_passes(), Flow::Continue);
    assert_ne!(h.mode, Mode::Steady);
}

#[test]
fn a_botched_ice_crossing_can_break_the_ice() {
    // A badly shaky run drops the party through the ice — a fatal ending.
    let mut g = mid_steady(7, super::state::CUMBERLAND_RIVER_AT);
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
    let mut g = mid_steady(7, super::state::CUMBERLAND_RIVER_AT);
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
    for seed in 0..40u64 {
        let g = play(seed, true);
        let end = g.outcome.expect("game ended without an outcome");
        // The journey can't run past the winter deadline.
        assert!(g.state.turn <= MAX_TURNS, "seed {seed} ran too long");
        // The final report never shows negative supplies.
        assert!(end.food >= 0 && end.bullets >= 0 && end.clothing >= 0);
        assert!(end.misc >= 0 && end.cash >= 0);
        assert!(end.miles >= 0 && end.miles <= TRAIL_MILES as i64);
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
        good_total += good.state.miles.min(TRAIL_MILES) as i64;
        if good.outcome.is_some_and(|e| e.won) {
            good_wins += 1;
        }
        bad_total += play(seed, false).state.miles.min(TRAIL_MILES) as i64;
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
