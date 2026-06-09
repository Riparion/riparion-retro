//! Engine tests: exact spec numbers from a fixed state, plus a multi-seed
//! scripted smoke test playing whole reigns through the public API.

use super::*;
use state::{Difficulty, Gender, Mode, START_YEAR};

fn fresh(difficulty: Difficulty) -> Game {
    let mut g = Game::new(12345);
    g.begin(
        "Reign".into(),
        "Santa Paravia".into(),
        Gender::Male,
        difficulty,
    );
    g
}

#[test]
fn initial_state_is_sane() {
    let g = fresh(Difficulty::Apprentice);
    let s = &g.state;
    assert_eq!(s.year, START_YEAR);
    assert_eq!(s.title_num, 0);
    assert_eq!(s.treasury, 1000);
    assert_eq!(s.land, 10000);
    assert_eq!(s.serfs, 2000);
    assert_eq!(g.mode, Mode::YearReport);
    // Reign of 20–55 years.
    assert!((START_YEAR + 20..=START_YEAR + 55).contains(&s.year_of_death));
    // Prices and demand were set by the first NewLandAndGrainPrices.
    assert!(s.land_price >= 1.0);
    assert!(s.grain_price >= 0);
    assert!(s.grain_demand > 0);
    assert!((0..=5).contains(&s.harvest));
    assert!((0..=50).contains(&s.rats));
    assert!(s.grain >= 0);
}

#[test]
fn limit10_caps_upside_only() {
    assert_eq!(limit10(5, 1), 5);
    assert_eq!(limit10(100, 1), 10); // capped at 10
    assert_eq!(limit10(12000, 5000), 2);
    assert_eq!(limit10(-30000, 5000), -6); // debt pulls the total down
    assert_eq!(limit10(0, 6000), 0);
}

#[test]
fn revenue_matches_hand_computed_spec() {
    let mut g = Game::new(1);
    let s = &mut g.state;
    s.nobles = 4;
    s.clergy = 5;
    s.merchants = 25;
    s.public_works = 1.0;
    s.customs_duty = 25;
    s.sales_tax = 10;
    s.income_tax = 5;
    s.justice = 2;
    s.title_num = 1;

    // Hand-computed in the plan: y = 1.10.
    let (customs, sales, income, justice, total) = g.projected_revenue();
    assert_eq!(customs, 436);
    assert_eq!(sales, 137);
    assert_eq!(income, 55);
    assert_eq!(justice, 100);
    assert_eq!(total, 728);
}

#[test]
fn release_bounds_are_twenty_to_eighty_percent() {
    let mut g = fresh(Difficulty::Apprentice);
    g.advance_from_report(); // → Market
    g.done_market(); // → Release
    assert_eq!(g.mode, Mode::Release);
    let grain = g.state.grain;
    assert_eq!(g.release_min(), grain / 5);
    assert_eq!(g.release_max(), grain - grain / 5);
    // Below the minimum is rejected.
    assert!(g.submit_release(g.release_min() - 1).is_err());
    assert_eq!(g.mode, Mode::Release);
    // Above the maximum is rejected.
    assert!(g.submit_release(g.release_max() + 1).is_err());
    assert_eq!(g.mode, Mode::Release);
}

#[test]
fn maxed_standing_wins_the_crown_at_apprentice() {
    let mut g = Game::new(7);
    g.begin(
        "Victor".into(),
        "Fiumaccio".into(),
        Gender::Male,
        Difficulty::Apprentice,
    );
    // Force a near-perfect standing, then run the title check via the public path.
    let s = &mut g.state;
    s.marketplaces = 20;
    s.palace = 20;
    s.cathedral = 20;
    s.mills = 20;
    s.treasury = 200_000;
    s.land = 100_000;
    s.merchants = 1000;
    s.nobles = 100;
    s.soldiers = 1000;
    s.clergy = 200;
    s.serfs = 50_000;
    s.public_works = 100.0;
    s.justice = 1;
    g.mode = Mode::Purchases;
    g.phase = Phase::TitleCheck;
    g.done_purchases();
    assert!(g.outcome.as_ref().map(|e| e.won).unwrap_or(false));
    assert_eq!(g.state.title_num, WIN_TITLE);
}

#[test]
fn deep_debt_triggers_bankruptcy_seizure() {
    let mut g = fresh(Difficulty::Apprentice);
    g.state.treasury = -100_000;
    g.state.title_num = 0;
    g.mode = Mode::Tax;
    g.phase = Phase::TaxResolve;
    g.set_customs_duty(0);
    g.set_sales_tax(0);
    g.set_income_tax(0);
    g.set_justice(1);
    g.done_tax();
    // Creditors seized the estate and reset the books.
    assert_eq!(g.state.treasury, 100);
    assert_eq!(g.state.land, 6000);
    assert_eq!(g.state.marketplaces, 0);
}

/// Drive a whole reign through the public API with a simple bot, asserting it
/// always terminates and never wedges a screen.
fn play_to_end(seed: u64, difficulty: Difficulty) -> Game {
    let mut g = Game::new(seed);
    g.begin("Bot".into(), "Torricella".into(), Gender::Female, difficulty);
    let mut steps = 0;
    while g.mode != Mode::GameOver {
        steps += 1;
        assert!(steps < 20_000, "reign failed to terminate (seed {seed})");
        match g.mode {
            Mode::YearReport => g.advance_from_report(),
            Mode::Market => {
                // Stockpile a little grain when it's cheap, then move on.
                let _ = g.buy_grain(500);
                g.done_market();
            }
            Mode::Release => {
                let _ = g.submit_release(g.release_max());
            }
            Mode::Tax => {
                // Keep moderate taxes; nudge justice fair for growth.
                g.set_justice(2);
                g.done_tax();
            }
            Mode::Purchases => {
                // Invest in whatever we can afford, in rising order.
                g.buy_marketplace();
                g.buy_soldiers();
                g.done_purchases();
            }
            Mode::Interaction => g.resolve(Response::Acknowledge),
            Mode::GameOver => break,
            Mode::Splash | Mode::NewGame => panic!("unexpected mode mid-reign"),
        }
        // Invariants that must hold throughout.
        assert!(g.state.grain >= 0, "negative grain (seed {seed})");
    }
    g
}

#[test]
fn reigns_terminate_across_seeds_and_difficulties() {
    for seed in 0..40u64 {
        for diff in Difficulty::ALL {
            let g = play_to_end(seed, diff);
            let end = g.outcome.expect("reign ended without an outcome");
            // Either crowned, or died within the lifespan.
            assert!(end.won || g.state.year >= g.state.year_of_death);
            assert!(end.years >= 0);
        }
    }
}
