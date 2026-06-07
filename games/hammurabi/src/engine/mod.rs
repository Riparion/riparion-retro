//! Pure game engine: no dioxus, no web — fully testable on the host.
//! A faithful port of HAMURABI from BASIC Computer Games (1978); line
//! numbers in comments refer to that listing.

pub mod scoring;
pub mod state;

use retro_kit::format::group_thousands;
use retro_kit::rng::GameRng;
use serde::{Deserialize, Serialize};

use scoring::EndGame;
use state::{LogLine, Mode, Phase, State};

/// Decision years in a full term (the year-11 report ends it, line 270).
pub const TERM_YEARS: i64 = 10;
/// Line 540: each person needs 20 bushels per year.
pub const BUSHELS_PER_PERSON: i64 = 20;
/// Line 455: each person can tend at most 10 acres (strictly fewer).
pub const ACRES_PER_WORKER: i64 = 10;
/// Line 510: 1 bushel of seed plants 2 acres (`INT(D/2)`).
pub fn seed_cost(acres: i64) -> i64 {
    acres / 2
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub mode: Mode,
    pub state: State,
    pub rng: GameRng,
    /// The steward's report scroll; the UI paces newly appended lines.
    pub log: Vec<LogLine>,
    pub outcome: Option<EndGame>,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            mode: Mode::Splash,
            state: State::initial(),
            rng: GameRng::from_seed(seed),
            log: Vec::new(),
            outcome: None,
        }
    }

    /// Take office: fresh ledger, the year-1 report already on the scroll.
    pub fn start(&mut self) {
        self.state = State::initial();
        self.log.clear();
        self.outcome = None;
        self.begin_year();
        self.mode = Mode::Year;
    }

    /// Feed the player's number to whichever prompt is open. Inputs are
    /// clamped (the soft keyboard has no "negative steward-dismissal" path).
    pub fn submit(&mut self, n: i64) {
        if self.mode != Mode::Year || self.outcome.is_some() {
            return;
        }
        match self.state.phase {
            Phase::BuyLand => self.buy_land(n),
            Phase::SellLand => self.sell_land(n),
            Phase::Feed => self.feed(n),
            Phase::Plant => self.plant(n),
        }
    }

    // ----- per-phase input bounds (the UI mirrors these; the engine is
    // authoritative and re-clamps) -----

    /// Line 322: spending is capped by the store.
    pub fn max_buy(&self) -> i64 {
        self.state.store / self.state.land_price.max(1)
    }

    /// Line 342: `Q < A` — the original never lets you sell your last acre.
    pub fn max_sell(&self) -> i64 {
        (self.state.acres - 1).max(0)
    }

    pub fn max_feed(&self) -> i64 {
        self.state.store
    }

    /// Lines 445/450/455: own the land, afford the seed, have the hands.
    /// Saturating: a corrupt save can't drive the bounds into overflow.
    pub fn max_plant(&self) -> i64 {
        self.state
            .acres
            .min(self.state.store.saturating_mul(2).saturating_add(1))
            .min(ACRES_PER_WORKER.saturating_mul(self.state.population) - 1)
            .max(0)
    }

    // ----- the four decisions (lines 320–510) -----

    fn buy_land(&mut self, acres: i64) {
        let acres = acres.clamp(0, self.max_buy());
        if acres == 0 {
            // Line 330: buying nothing opens the offer to sell.
            self.state.phase = Phase::SellLand;
            return;
        }
        self.state.acres += acres;
        self.state.store -= acres * self.state.land_price;
        self.state.phase = Phase::Feed; // line 334 skips the sell prompt
    }

    fn sell_land(&mut self, acres: i64) {
        let acres = acres.clamp(0, self.max_sell());
        self.state.acres -= acres;
        self.state.store += acres * self.state.land_price;
        self.state.phase = Phase::Feed;
    }

    fn feed(&mut self, bushels: i64) {
        let bushels = bushels.clamp(0, self.max_feed());
        self.state.store -= bushels;
        self.state.fed_bushels = bushels;
        self.state.phase = Phase::Plant;
    }

    fn plant(&mut self, acres: i64) {
        let acres = acres.clamp(0, self.max_plant());
        self.state.store -= seed_cost(acres);
        self.resolve_year(acres);
    }

    /// Lines 511–555: harvest, rats, immigration, feeding, plague roll —
    /// then either the next report, impeachment, or the final judgment.
    fn resolve_year(&mut self, planted: i64) {
        let st = &mut self.state;
        // Harvest (511–515): yield 1..=5 bushels per acre.
        st.bushels_per_acre = roll_1_to_5(&mut self.rng);
        let harvest = planted * st.bushels_per_acre;
        // Rats (521–530): on an even roll they eat store/roll — of the
        // store as it stands after feeding and seed, before the harvest.
        let c = roll_1_to_5(&mut self.rng);
        st.rats_ate = rats_eaten(c, st.store);
        st.store = st.store - st.rats_ate + harvest;
        // Immigration (531–533), applied at the next report.
        let c = roll_1_to_5(&mut self.rng);
        st.immigrants = immigrants(c, st.acres, st.store, st.population);
        // Feeding (540–555): 20 bushels keep one person alive.
        let fed_people = st.fed_bushels / BUSHELS_PER_PERSON;
        if st.population < fed_people {
            // Line 550 → 210: surplus food, nobody starves — and the
            // original skips the P1 average update entirely.
            st.starved = 0;
        } else {
            let starved = st.population - fed_people;
            if starved as f64 > 0.45 * st.population as f64 {
                // Line 552 → 560: starved more than 45% in one year.
                st.starved = starved;
                self.impeach(starved);
                return;
            }
            // Line 553: running average of percent starved per year.
            let z = st.year as f64;
            st.avg_starved_pct = ((z - 1.0) * st.avg_starved_pct
                + starved as f64 * 100.0 / st.population.max(1) as f64)
                / z;
            st.population = fed_people;
            st.total_starved += starved;
            st.starved = starved;
        }
        // Plague roll for next year (542): Q=INT(10*(2*RND-0.3)), Q<=0 hits.
        let q = (10.0 * (2.0 * self.rng.uniform() - 0.3)).floor() as i64;
        self.state.plague_pending = q <= 0;
        self.begin_year();
    }

    /// Lines 215–312: the steward's report opens each year — immigration
    /// and plague land here, then the land price is set.
    fn begin_year(&mut self) {
        let st = &mut self.state;
        st.year += 1;
        st.fed_bushels = 0;
        let mut lines = vec![
            LogLine::head("HAMURABI:  I BEG TO REPORT TO YOU,"),
            LogLine::line(format!(
                "IN YEAR {}, {} PEOPLE STARVED, {} CAME TO THE CITY,",
                st.year, st.starved, st.immigrants
            )),
        ];
        st.population += st.immigrants; // line 218
        if st.plague_pending {
            st.population /= 2; // line 228
            st.plague_pending = false;
            lines.push(LogLine::alert("A HORRIBLE PLAGUE STRUCK!  HALF THE PEOPLE DIED."));
        }
        lines.push(LogLine::line(format!("POPULATION IS NOW {}", group_thousands(st.population))));
        lines.push(LogLine::line(format!(
            "THE CITY NOW OWNS {} ACRES.",
            group_thousands(st.acres)
        )));
        lines.push(LogLine::line(format!(
            "YOU HARVESTED {} BUSHELS PER ACRE.",
            st.bushels_per_acre
        )));
        lines.push(LogLine::line(format!("THE RATS ATE {} BUSHELS.", group_thousands(st.rats_ate))));
        lines.push(LogLine::line(format!(
            "YOU NOW HAVE {} BUSHELS IN STORE.",
            group_thousands(st.store)
        )));
        if st.year > TERM_YEARS {
            // Line 270: the year-11 report is the last thing the city hears.
            self.log.extend(lines);
            self.outcome = Some(scoring::evaluate(&self.state, &mut self.rng));
            return;
        }
        st.land_price = self.rng.ri(10) + 17; // line 310
        lines.push(LogLine::line(format!(
            "LAND IS TRADING AT {} BUSHELS PER ACRE.",
            st.land_price
        )));
        st.phase = Phase::BuyLand;
        self.log.extend(lines);
    }

    fn impeach(&mut self, starved: i64) {
        self.log.extend([
            LogLine::alert(format!("YOU STARVED {} PEOPLE IN ONE YEAR!!!", group_thousands(starved))),
            LogLine::alert("DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE NOT ONLY"),
            LogLine::alert("BEEN IMPEACHED AND THROWN OUT OF OFFICE BUT YOU HAVE"),
            LogLine::alert("ALSO BEEN DECLARED NATIONAL FINK!!!!"),
        ]);
        self.outcome = Some(scoring::impeached(&self.state, starved));
    }

    /// The term has been decided — the current playback is the last one.
    pub fn decided(&self) -> bool {
        self.outcome.is_some()
    }

    /// Flip to the reckoning screen once playback has finished.
    pub fn finish_year(&mut self) {
        if self.outcome.is_some() {
            self.mode = Mode::GameOver;
        }
    }

    /// Clamp a deserialized ledger to sane ranges. Engine-driven play never
    /// leaves them (population >= 1, store bounded by the 10-year term),
    /// but a hand-edited or corrupted localStorage save is accepted as-is
    /// by serde and could otherwise drive the arithmetic into overflow.
    pub fn sanitize(&mut self) {
        // Far beyond any reachable game state, far below any overflow.
        const CAP: i64 = 1_000_000_000;
        let st = &mut self.state;
        st.year = st.year.clamp(0, TERM_YEARS + 1);
        st.population = st.population.clamp(0, CAP);
        st.store = st.store.clamp(0, CAP);
        st.acres = st.acres.clamp(1, CAP);
        st.bushels_per_acre = st.bushels_per_acre.clamp(0, 5);
        st.rats_ate = st.rats_ate.clamp(0, CAP);
        st.immigrants = st.immigrants.clamp(0, CAP);
        st.starved = st.starved.clamp(0, CAP);
        st.fed_bushels = st.fed_bushels.clamp(0, CAP);
        st.total_starved = st.total_starved.clamp(0, CAP);
        st.land_price = st.land_price.clamp(17, 26);
        if !st.avg_starved_pct.is_finite() {
            st.avg_starved_pct = 0.0;
        }
    }
}

/// Subroutine 800: `C=INT(RND(1)*5)+1` — the harvest/rats/immigration die.
fn roll_1_to_5(rng: &mut GameRng) -> i64 {
    rng.ri(5) + 1
}

/// Lines 522–525: rats eat `INT(S/C)` only when the roll is even.
pub fn rats_eaten(roll: i64, store: i64) -> i64 {
    if roll % 2 == 0 {
        store / roll
    } else {
        0
    }
}

/// Line 533: `I=INT(C*(20*A+S)/P/100+1)`. The divisor is guarded so a
/// corrupt zero-population save can't produce an infinity (population is
/// provably >= 1 in engine-driven play).
pub fn immigrants(roll: i64, acres: i64, store: i64, population: i64) -> i64 {
    (roll as f64 * (20.0 * acres as f64 + store as f64) / population.max(1) as f64 / 100.0 + 1.0)
        .floor() as i64
}

impl Default for Game {
    fn default() -> Self {
        Self::new(retro_kit::rng::random_seed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scoring::Verdict;

    fn started(seed: u64) -> Game {
        let mut game = Game::new(seed);
        game.start();
        game
    }

    fn log_text(game: &Game) -> Vec<&str> {
        game.log.iter().map(|l| l.text.as_str()).collect()
    }

    #[test]
    fn first_report_matches_the_seeded_ledger() {
        let game = started(1);
        let text = log_text(&game);
        assert!(text.contains(&"IN YEAR 1, 0 PEOPLE STARVED, 5 CAME TO THE CITY,"));
        assert!(text.contains(&"POPULATION IS NOW 100"));
        assert!(text.contains(&"THE CITY NOW OWNS 1,000 ACRES."));
        assert!(text.contains(&"YOU HARVESTED 3 BUSHELS PER ACRE."));
        assert!(text.contains(&"THE RATS ATE 200 BUSHELS."));
        assert!(text.contains(&"YOU NOW HAVE 2,800 BUSHELS IN STORE."));
        assert_eq!(game.state.population, 100);
        assert_eq!(game.state.phase, Phase::BuyLand);
        assert_eq!(game.mode, Mode::Year);
    }

    #[test]
    fn land_trades_between_17_and_26() {
        for seed in 0..100 {
            let game = started(seed);
            assert!((17..=26).contains(&game.state.land_price));
        }
    }

    #[test]
    fn buying_land_pays_the_price_and_skips_the_sell_prompt() {
        let mut game = started(1);
        let price = game.state.land_price;
        game.submit(50);
        assert_eq!(game.state.acres, 1050);
        assert_eq!(game.state.store, 2800 - 50 * price);
        assert_eq!(game.state.phase, Phase::Feed);
    }

    #[test]
    fn buy_is_clamped_to_what_the_store_affords() {
        let mut game = started(1);
        let price = game.state.land_price;
        let max = 2800 / price;
        game.submit(1_000_000);
        assert_eq!(game.state.acres, 1000 + max);
        assert!(game.state.store >= 0);
        assert!(game.state.store < price, "change must be less than one acre's price");
    }

    #[test]
    fn buying_nothing_offers_to_sell() {
        let mut game = started(1);
        game.submit(0);
        assert_eq!(game.state.phase, Phase::SellLand);
        let price = game.state.land_price;
        game.submit(100);
        assert_eq!(game.state.acres, 900);
        assert_eq!(game.state.store, 2800 + 100 * price);
        assert_eq!(game.state.phase, Phase::Feed);
    }

    #[test]
    fn the_last_acre_can_never_be_sold() {
        // Line 342's strict `Q < A`.
        let mut game = started(1);
        game.submit(0);
        game.submit(i64::MAX);
        assert_eq!(game.state.acres, 1);
    }

    #[test]
    fn feeding_is_clamped_to_the_store() {
        let mut game = started(1);
        game.submit(0);
        game.submit(0);
        game.submit(5_000);
        assert_eq!(game.state.fed_bushels, 2800);
        assert_eq!(game.state.store, 0);
        assert_eq!(game.state.phase, Phase::Plant);
    }

    #[test]
    fn planting_bounds_land_seed_and_labor() {
        let mut game = started(1);
        // Plenty of land and people, little seed: 2S+1 binds.
        game.state.store = 100;
        game.state.acres = 5_000;
        game.state.population = 100;
        assert_eq!(game.max_plant(), 201);
        // Little land: A binds.
        game.state.acres = 50;
        assert_eq!(game.max_plant(), 50);
        // Few people: 10P-1 binds (line 455's strict `D < 10*P`).
        game.state.acres = 5_000;
        game.state.population = 3;
        assert_eq!(game.max_plant(), 29);
    }

    #[test]
    fn planting_costs_one_bushel_per_two_acres() {
        let mut game = started(1);
        game.submit(0); // buy
        game.submit(0); // sell
        game.submit(2000); // feed
        let store_before = game.state.store; // 800
        let planted = 101i64;
        game.submit(planted);
        // The harvest landed in the store afterwards; back it out along
        // with the rats to check the seed deduction: floor(101/2) = 50.
        let st = &game.state;
        let after_seed = st.store + st.rats_ate - planted * st.bushels_per_acre;
        assert_eq!(after_seed, store_before - 50);
    }

    #[test]
    fn rats_eat_only_on_even_rolls() {
        assert_eq!(rats_eaten(1, 1000), 0);
        assert_eq!(rats_eaten(2, 1000), 500);
        assert_eq!(rats_eaten(3, 1000), 0);
        assert_eq!(rats_eaten(4, 1000), 250);
        assert_eq!(rats_eaten(5, 1000), 0);
        assert_eq!(rats_eaten(4, 1001), 250); // INT(S/C)
    }

    #[test]
    fn immigration_matches_line_533() {
        // I = INT(C*(20*A+S)/P/100 + 1) on the worked example:
        // C=1, A=1000, S=2800, P=100 → INT(22800/100/100 + 1) = 3.
        assert_eq!(immigrants(1, 1000, 2800, 100), 3);
        assert_eq!(immigrants(5, 1000, 2800, 100), 12);
        assert_eq!(immigrants(1, 1, 0, 1000), 1); // never less than 1
        // INT(3*(20*500+150)/7/100 + 1) = INT(44.5) = 44.
        assert_eq!(immigrants(3, 500, 150, 7), 44);
    }

    /// Run one resolution with a fixed population/feed and no planting, then
    /// report what the engine recorded. Harvest/rats/immigration still roll,
    /// but starvation depends only on the fed bushels and the population.
    fn resolve_with(population: i64, fed: i64, avg0: f64, year: i64) -> Game {
        let mut game = started(1);
        game.state.population = population;
        game.state.avg_starved_pct = avg0;
        game.state.year = year;
        game.state.fed_bushels = fed;
        game.state.phase = Phase::Plant;
        game.submit(0); // plant nothing → resolve
        game
    }

    #[test]
    fn exactly_45_percent_starved_keeps_the_throne() {
        // 100 people, 55 fed → 45 starved; 45 > 45.0 is false (line 552).
        let game = resolve_with(100, 1100, 0.0, 1);
        assert!(game.outcome.is_none());
        assert_eq!(game.state.year, 2);
        // The report logs last year's toll.
        assert!(game.log.iter().any(|l| l.text.starts_with("IN YEAR 2, 45 PEOPLE STARVED,")));
        assert!((game.state.avg_starved_pct - 45.0).abs() < 1e-9);
        assert_eq!(game.state.total_starved, 45);
    }

    #[test]
    fn starving_46_percent_is_impeachment() {
        let game = resolve_with(100, 1080, 0.0, 1);
        let end = game.outcome.expect("thrown out of office");
        assert_eq!(end.verdict, Verdict::Impeached);
        assert_eq!(end.starved_final_year, 46);
        assert_eq!(end.score, 0);
        assert_eq!(game.mode, Mode::Year, "mode flips only in finish_year");
        assert!(game.log.iter().any(|l| l.text == "YOU STARVED 46 PEOPLE IN ONE YEAR!!!"));
    }

    #[test]
    fn surplus_food_skips_the_average_update() {
        // Line 550's `IF P<C THEN 210` bypasses line 553 entirely.
        let game = resolve_with(10, 400, 7.0, 3);
        assert_eq!(game.state.starved, 0);
        assert!((game.state.avg_starved_pct - 7.0).abs() < 1e-9, "P1 must be untouched");
    }

    #[test]
    fn exact_feeding_averages_in_a_zero() {
        // P == C runs line 553 with D = 0: P1 = ((Z-1)*P1 + 0)/Z.
        let game = resolve_with(10, 200, 7.0, 3);
        assert_eq!(game.state.starved, 0);
        assert!((game.state.avg_starved_pct - 14.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn starvation_average_is_a_running_mean() {
        // Year 1: 100 people, 95 fed → 5 starved → P1 = 5.0.
        let game = resolve_with(100, 1900, 0.0, 1);
        assert!((game.state.avg_starved_pct - 5.0).abs() < 1e-9);
        assert_eq!(game.state.total_starved, 5);
        // Year 2 with 10% starved: P1 = (5 + 10)/2.
        let game = resolve_with(100, 1800, 5.0, 2);
        assert!((game.state.avg_starved_pct - 7.5).abs() < 1e-9);
    }

    #[test]
    fn plague_strikes_about_one_year_in_five_and_halves_the_city() {
        let mut rng = GameRng::from_seed(9);
        let mut hits = 0;
        let n = 100_000;
        for _ in 0..n {
            let q = (10.0 * (2.0 * rng.uniform() - 0.3)).floor() as i64;
            if q <= 0 {
                hits += 1;
            }
        }
        let rate = hits as f64 / n as f64;
        assert!((0.19..0.21).contains(&rate), "plague rate {rate}");

        // Halving floors, and lands after immigration (lines 218→228).
        let mut game = started(1);
        game.state.population = 90;
        game.state.immigrants = 11;
        game.state.plague_pending = true;
        game.begin_year();
        assert_eq!(game.state.population, 50);
        assert!(game.log.iter().any(|l| l.alert && l.text.contains("PLAGUE")));
    }

    #[test]
    fn the_term_ends_with_the_year_11_report() {
        let mut game = started(3);
        game.state.year = 10;
        game.state.phase = Phase::Plant;
        game.state.fed_bushels = game.state.population * BUSHELS_PER_PERSON;
        game.state.store = game.state.fed_bushels + 100;
        game.submit(0);
        assert_eq!(game.state.year, 11);
        let end = game.outcome.as_ref().expect("term over");
        assert_eq!(end.years, 10);
        assert!(game.log.iter().any(|l| l.text.starts_with("IN YEAR 11,")));
        // No year-11 land price line — the city hears the report and stops.
        assert!(!log_text(&game).last().unwrap().starts_with("LAND IS TRADING"));
        game.finish_year();
        assert_eq!(game.mode, Mode::GameOver);
    }

    #[test]
    fn submit_after_the_term_is_decided_is_inert() {
        let mut game = resolve_with(100, 1080, 0.0, 1);
        assert!(game.decided());
        let snapshot = game.clone();
        game.submit(500);
        assert_eq!(game, snapshot);
    }

    #[test]
    fn corrupt_saves_are_sanitized_and_safe() {
        // A hand-edited/corrupted save must not drive the arithmetic into
        // division-by-zero infinities or multiplication overflow.
        let mut game = started(1);
        game.state.population = -5;
        game.state.store = i64::MAX;
        game.state.acres = 0;
        game.state.avg_starved_pct = f64::NAN;
        game.sanitize();
        assert_eq!(game.state.population, 0);
        assert_eq!(game.state.acres, 1);
        assert_eq!(game.state.avg_starved_pct, 0.0);
        assert!(game.max_plant() >= 0); // no overflow panic / negative cap
        // Guarded divisors stay finite even at population 0.
        assert!(immigrants(5, game.state.acres, game.state.store, 0) >= 1);
        let mut rng = GameRng::from_seed(1);
        let end = scoring::evaluate(&game.state, &mut rng);
        assert!(end.acres_per_person.is_finite());
        // ...and a turn resolves without panicking.
        game.state.phase = Phase::Plant;
        game.submit(0);
    }

    #[test]
    fn max_plant_saturates_instead_of_overflowing() {
        let mut game = started(1);
        game.state.store = i64::MAX;
        game.state.population = i64::MAX;
        game.state.acres = 50;
        assert_eq!(game.max_plant(), 50);
    }

    #[test]
    fn save_round_trip_preserves_the_game() {
        let mut game = started(5);
        game.submit(10); // buy
        game.submit(1800); // feed
        let json = serde_json::to_string(&game).expect("serialize");
        let back: Game = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(game, back);
    }

    /// Play whole terms through the public API across many seeds with a
    /// sensible regency: never buy, sell land to cover a famine year, feed
    /// everyone if possible, plant the maximum. Assert the invariants hold
    /// and every game actually ends.
    #[test]
    fn scripted_terms_stay_sane_across_seeds() {
        let mut finished = 0;
        let mut impeached_count = 0;
        for seed in 0..200 {
            let mut game = started(seed);
            for _ in 0..200 {
                if game.decided() {
                    break;
                }
                let st = &game.state;
                let n = match st.phase {
                    Phase::BuyLand => 0,
                    Phase::SellLand => {
                        // Sell just enough land to feed everyone this year.
                        let deficit = st.population * BUSHELS_PER_PERSON - st.store;
                        if deficit > 0 {
                            (deficit + st.land_price - 1) / st.land_price
                        } else {
                            0
                        }
                    }
                    Phase::Feed => game.max_feed().min(st.population * BUSHELS_PER_PERSON),
                    Phase::Plant => game.max_plant(),
                };
                game.submit(n);
                let st = &game.state;
                assert!(st.store >= 0, "seed {seed}: store went negative");
                assert!(st.acres >= 1, "seed {seed}: city lost all land");
                assert!(st.population >= 1, "seed {seed}: population {}", st.population);
                assert!(st.year <= 11, "seed {seed}: year ran past the term");
                assert!((17..=26).contains(&st.land_price), "seed {seed}: price");
                assert!((1..=5).contains(&st.bushels_per_acre), "seed {seed}: yield");
            }
            let end = game.outcome.as_ref().unwrap_or_else(|| panic!("seed {seed}: never ended"));
            match end.verdict {
                Verdict::Impeached => impeached_count += 1,
                _ => {
                    finished += 1;
                    assert_eq!(end.years, 10);
                }
            }
            game.finish_year();
            assert_eq!(game.mode, Mode::GameOver);
        }
        // The strategy is reasonable: most reigns should survive the term.
        assert!(finished > impeached_count, "{finished} finished, {impeached_count} impeached");
    }
}
