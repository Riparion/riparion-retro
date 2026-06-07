//! End-of-term judgment (lines 860–975) and the score scheme — the original
//! kept no scores, so ours rewards the verdict tier plus a thriving city.

use retro_kit::rng::GameRng;
use serde::{Deserialize, Serialize};

use super::state::State;
use super::TERM_YEARS;

/// How history remembers the reign. The first four are the original's
/// end-of-term tiers; `Impeached` is the mid-term >45%-starved ouster
/// (which the original sends to the same "national fink" text).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    Fantastic,
    NotTooBad,
    HeavyHanded,
    Fink,
    Impeached,
}

impl Verdict {
    pub fn label(&self) -> &'static str {
        match self {
            Verdict::Fantastic => "A fantastic performance",
            Verdict::NotTooBad => "Not too bad at all",
            Verdict::HeavyHanded => "Heavy-handed",
            Verdict::Fink => "National fink",
            Verdict::Impeached => "Impeached",
        }
    }

    /// A completed, non-disgraced term is worth recording.
    pub fn honourable(&self) -> bool {
        matches!(self, Verdict::Fantastic | Verdict::NotTooBad | Verdict::HeavyHanded)
    }
}

/// The final reckoning, built once when the term ends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGame {
    pub verdict: Verdict,
    /// P1: average percent of the population starved per year.
    pub avg_starved_pct: f64,
    /// D1: total people starved over the term.
    pub total_starved: i64,
    /// L = A/P at the end (the original notes you started with 10).
    pub acres_per_person: f64,
    pub population: i64,
    pub acres: i64,
    /// Decision years actually served (10 for a full term).
    pub years: i64,
    /// The middle tier's `INT(P*.8*RND(1))` would-be assassins.
    pub haters: i64,
    /// People starved in the fatal year (Impeached only).
    pub starved_final_year: i64,
    pub score: i64,
    /// Guards the once-only high-score record in the autosave effect.
    pub recorded: bool,
}

fn tier_bonus(verdict: Verdict) -> i64 {
    match verdict {
        Verdict::Fantastic => 10_000,
        Verdict::NotTooBad => 5_000,
        Verdict::HeavyHanded => 2_000,
        Verdict::Fink | Verdict::Impeached => 0,
    }
}

/// Disgraced reigns score zero; otherwise the tier dominates and a bigger,
/// better-fed city breaks ties.
pub fn score(verdict: Verdict, population: i64, acres: i64) -> i64 {
    if !verdict.honourable() {
        return 0;
    }
    tier_bonus(verdict) + population * 10 + acres
}

/// Lines 860–900: judge a completed 10-year term.
pub fn evaluate(st: &State, rng: &mut GameRng) -> EndGame {
    let p1 = st.avg_starved_pct;
    let l = st.acres as f64 / st.population.max(1) as f64;
    let verdict = if p1 > 33.0 || l < 7.0 {
        Verdict::Fink
    } else if p1 > 10.0 || l < 9.0 {
        Verdict::HeavyHanded
    } else if p1 > 3.0 || l < 10.0 {
        Verdict::NotTooBad
    } else {
        Verdict::Fantastic
    };
    // Line 965: INT(P*.8*RND(1)) — rolled only on the middle tier's text.
    let haters = if verdict == Verdict::NotTooBad {
        (st.population as f64 * 0.8 * rng.uniform()).floor() as i64
    } else {
        0
    };
    EndGame {
        verdict,
        avg_starved_pct: p1,
        total_starved: st.total_starved,
        acres_per_person: l,
        population: st.population,
        acres: st.acres,
        years: TERM_YEARS,
        haters,
        starved_final_year: 0,
        score: score(verdict, st.population, st.acres),
        recorded: false,
    }
}

/// Lines 560–567: starved more than 45% in one year — thrown out mid-term.
pub fn impeached(st: &State, starved: i64) -> EndGame {
    EndGame {
        verdict: Verdict::Impeached,
        avg_starved_pct: st.avg_starved_pct,
        total_starved: st.total_starved + starved,
        acres_per_person: st.acres as f64 / st.population.max(1) as f64,
        population: st.population - starved,
        acres: st.acres,
        years: st.year,
        haters: 0,
        starved_final_year: starved,
        score: 0,
        recorded: false,
    }
}

/// One row of the local hall of rulers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighScore {
    pub score: i64,
    pub verdict: String,
    pub population: i64,
    pub acres: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term_state(avg_starved_pct: f64, population: i64, acres: i64) -> State {
        State { avg_starved_pct, population, acres, year: 11, ..State::initial() }
    }

    fn verdict_of(p1: f64, pop: i64, acres: i64) -> Verdict {
        let mut rng = GameRng::from_seed(1);
        evaluate(&term_state(p1, pop, acres), &mut rng).verdict
    }

    #[test]
    fn tier_boundaries_match_lines_880_896() {
        // Worst tier: P1 > 33 OR L < 7.
        assert_eq!(verdict_of(33.1, 100, 2000), Verdict::Fink);
        assert_eq!(verdict_of(0.0, 100, 699), Verdict::Fink);
        // Exactly at the thresholds falls through to the next tier.
        assert_eq!(verdict_of(33.0, 100, 700), Verdict::HeavyHanded);
        // Heavy-handed: P1 > 10 OR L < 9.
        assert_eq!(verdict_of(10.1, 100, 2000), Verdict::HeavyHanded);
        assert_eq!(verdict_of(0.0, 100, 899), Verdict::HeavyHanded);
        // Middle: P1 > 3 OR L < 10.
        assert_eq!(verdict_of(10.0, 100, 900), Verdict::NotTooBad);
        assert_eq!(verdict_of(3.1, 100, 2000), Verdict::NotTooBad);
        assert_eq!(verdict_of(0.0, 100, 999), Verdict::NotTooBad);
        // Best: P1 <= 3 AND L >= 10.
        assert_eq!(verdict_of(3.0, 100, 1000), Verdict::Fantastic);
        assert_eq!(verdict_of(0.0, 95, 1000), Verdict::Fantastic);
    }

    #[test]
    fn haters_stay_under_eighty_percent_of_population() {
        let st = term_state(5.0, 100, 1500); // NotTooBad
        for seed in 0..200 {
            let mut rng = GameRng::from_seed(seed);
            let end = evaluate(&st, &mut rng);
            assert_eq!(end.verdict, Verdict::NotTooBad);
            assert!((0..80).contains(&end.haters), "haters {}", end.haters);
        }
    }

    #[test]
    fn disgraced_terms_score_zero() {
        assert_eq!(score(Verdict::Fink, 200, 2000), 0);
        assert_eq!(score(Verdict::Impeached, 200, 2000), 0);
        assert!(score(Verdict::HeavyHanded, 1, 1) > 0);
        // For the same city, a better verdict always outscores a worse one.
        assert!(score(Verdict::Fantastic, 100, 1000) > score(Verdict::NotTooBad, 100, 1000));
        assert!(score(Verdict::NotTooBad, 100, 1000) > score(Verdict::HeavyHanded, 100, 1000));
    }

    #[test]
    fn impeachment_reckoning_counts_the_fatal_year() {
        let st = State { year: 4, population: 100, acres: 800, total_starved: 12, ..State::initial() };
        let end = impeached(&st, 46);
        assert_eq!(end.verdict, Verdict::Impeached);
        assert_eq!(end.starved_final_year, 46);
        assert_eq!(end.total_starved, 58);
        assert_eq!(end.population, 54);
        assert_eq!(end.years, 4);
        assert_eq!(end.score, 0);
    }
}
