//! Pure game engine: no dioxus, no web — fully testable on the host.
//! A faithful port of FUR TRADER from BASIC Computer Games (1976); line
//! numbers in comments refer to that listing. One documented deviation:
//! the original never sets a fox price at Fort Stadacona (line 1205–1207
//! roll only mink/ermine/beaver), so fox would sell at a stale price or
//! $0.00 — this port rolls one.

pub mod scoring;
pub mod state;

pub use retro_kit::format::fmt_dollars;
use retro_kit::rng::GameRng;
use serde::{Deserialize, Serialize};

use scoring::EndGame;
use state::{
    fort_info, Fort, Fur, LogLine, Mode, State, FURS_PER_YEAR, FUR_ORDER, MIN_EXPEDITION_COST,
};

/// The BASIC price idiom `INT(x*10^2+.5)/10^2`: round to the nearest cent.
pub fn round2(x: f64) -> f64 {
    (x * 100.0 + 0.5).floor() / 100.0
}

/// Lines 261–262, rolled at the start of each year: (ermine, beaver).
/// Fort New York sells ermine and beaver at these carried-over prices.
pub fn start_prices(rng: &mut GameRng) -> (f64, f64) {
    let ermine = round2(0.15 * rng.uniform() + 0.95); // line 261: $0.95–$1.10
    let beaver = round2(0.25 * rng.uniform() + 1.00); // line 262: $1.00–$1.25
    (ermine, beaver)
}

/// Per-fort pelt prices, indexed by `Fur` ([mink, beaver, ermine, fox]).
/// Rolls in the exact BASIC line order — the contract for seeded tests.
pub fn fort_prices(fort: Fort, start_ermine: f64, start_beaver: f64, rng: &mut GameRng) -> [f64; 4] {
    match fort {
        Fort::Hochelaga => {
            // Lines 1174–1177: mink, ermine, beaver, fox.
            let mink = round2(0.2 * rng.uniform() + 0.7);
            let ermine = round2(0.2 * rng.uniform() + 0.65);
            let beaver = round2(0.2 * rng.uniform() + 0.75);
            let fox = round2(0.2 * rng.uniform() + 0.8);
            [mink, beaver, ermine, fox]
        }
        Fort::Stadacona => {
            // Lines 1205–1207: mink, ermine, beaver. The original never
            // rolls fox here (the famous bug); this port gives it the
            // Hochelaga-style $0.80–$1.00 range.
            let mink = round2(0.3 * rng.uniform() + 0.85);
            let ermine = round2(0.15 * rng.uniform() + 0.8);
            let beaver = round2(0.2 * rng.uniform() + 0.9);
            let fox = round2(0.2 * rng.uniform() + 0.8);
            [mink, beaver, ermine, fox]
        }
        Fort::NewYork => {
            // Lines 1260/1263: mink, fox. Ermine and beaver sell at the
            // start-of-year prices — intentional ("highest value").
            let mink = round2(0.15 * rng.uniform() + 1.05);
            let fox = round2(0.25 * rng.uniform() + 1.1);
            [mink, start_beaver, start_ermine, fox]
        }
    }
}

/// What happened on the trip (lines 1209–1238 and 1270–1309).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    Safe,
    /// Stadacona p<=2: beaver left at the portage, stolen.
    BeaverStolen,
    /// Stadacona p<=8 / New York p<=8: every pelt lost.
    AllFursLost,
    /// Stadacona p<=10: fox pelts not cured, unsellable.
    FoxUncured,
    /// New York p<=2: the Iroquois ambush — the game ends.
    Killed,
    /// New York p<=10: mink and beaver prices halved.
    FursDamaged,
}

/// `P=INT(10*RND(1))+1` then the threshold ladder (1210–1215 / 1271–1274).
pub fn fort_event(fort: Fort, rng: &mut GameRng) -> Event {
    let p = match fort {
        Fort::Hochelaga => return Event::Safe, // no event roll at all
        Fort::Stadacona | Fort::NewYork => rng.ri(10) + 1,
    };
    match (fort, p) {
        (Fort::Stadacona, ..=2) => Event::BeaverStolen,
        (Fort::Stadacona, ..=6) => Event::Safe,
        (Fort::Stadacona, ..=8) => Event::AllFursLost,
        (Fort::Stadacona, _) => Event::FoxUncured,
        (Fort::NewYork, ..=2) => Event::Killed,
        (Fort::NewYork, ..=6) => Event::Safe,
        (Fort::NewYork, ..=8) => Event::AllFursLost,
        (Fort::NewYork, _) => Event::FursDamaged,
        (Fort::Hochelaga, _) => unreachable!(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub mode: Mode,
    pub state: State,
    pub rng: GameRng,
    /// The expedition journal; the UI paces newly appended lines.
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

    /// Set out from Lake Ontario: fresh books, year 1 begun.
    pub fn start(&mut self) {
        self.state = State::initial();
        self.log.clear();
        self.outcome = None;
        self.begin_year();
        self.mode = Mode::Allocate;
    }

    /// A new trading year: fresh 190 furs, fresh default prices — unless
    /// the company can no longer afford the cheapest expedition.
    fn begin_year(&mut self) {
        if self.state.savings < MIN_EXPEDITION_COST {
            self.log.push(LogLine::alert(format!(
                "YOU CANNOT RAISE THE {} TO OUTFIT ANOTHER EXPEDITION.",
                fmt_dollars(MIN_EXPEDITION_COST)
            )));
            self.log.push(LogLine::alert("YOUR TRADING DAYS ARE OVER."));
            self.outcome = Some(scoring::bankrupt(&self.state));
            return;
        }
        let st = &mut self.state;
        st.year += 1;
        st.furs = [0; 4];
        st.cursor = 0;
        st.fort = None;
        st.last_sale = None;
        let (ermine, beaver) = start_prices(&mut self.rng);
        st.start_ermine_price = ermine;
        st.start_beaver_price = beaver;
    }

    // ----- allocation (lines 330–350; the UI mirrors these bounds, the
    // engine is authoritative and re-clamps) -----

    /// Furs not yet assigned to a pelt type this year.
    pub fn remaining(&self) -> i64 {
        FURS_PER_YEAR - self.state.allocated()
    }

    /// The pelt the allocation prompt is on, if any are left to answer.
    pub fn current_fur(&self) -> Option<Fur> {
        FUR_ORDER.get(self.state.cursor).copied()
    }

    /// Answer the current "HOW MANY ... PELTS DO YOU HAVE" prompt. Clamped
    /// to the unassigned remainder — line 344's cheat path can't happen.
    pub fn submit_fur(&mut self, n: i64) {
        if self.mode != Mode::Allocate || self.decided() || self.state.cursor >= FUR_ORDER.len() {
            return;
        }
        let fur = FUR_ORDER[self.state.cursor];
        self.state.furs[fur as usize] = n.clamp(0, self.remaining());
        self.state.cursor += 1;
    }

    /// Start the distribution over.
    pub fn redo_allocation(&mut self) {
        if self.mode != Mode::Allocate || self.decided() {
            return;
        }
        self.state.furs = [0; 4];
        self.state.cursor = 0;
    }

    /// All four pelts answered: on to the fort choice. Like the original,
    /// taking fewer than 190 is allowed (the rest stay home unsold).
    pub fn finish_allocation(&mut self) {
        if self.mode != Mode::Allocate || self.decided() || self.state.cursor < FUR_ORDER.len() {
            return;
        }
        self.mode = Mode::FortSelect;
    }

    // ----- the fort (lines 1100–1156) -----

    /// Tentative pick: shows the route description, nothing committed yet.
    pub fn choose_fort(&mut self, fort: Fort) {
        if self.mode != Mode::FortSelect || self.decided() {
            return;
        }
        self.state.fort = Some(fort);
        self.mode = Mode::FortConfirm;
    }

    /// "DO YOU WANT TO TRADE AT ANOTHER FORT?" — YES (line 1129/1144/1155).
    pub fn change_fort(&mut self) {
        if self.mode != Mode::FortConfirm || self.decided() {
            return;
        }
        self.state.fort = None;
        self.mode = Mode::FortSelect;
    }

    /// — NO: commit the trip. Pay the outfitting cost (you pay even when
    /// the trip goes wrong), roll prices then the event in BASIC order,
    /// sell what survives, and journal the lot for paced playback.
    pub fn confirm_fort(&mut self) {
        if self.mode != Mode::FortConfirm || self.decided() {
            return;
        }
        let Some(fort) = self.state.fort else {
            return;
        };
        let info = fort_info(fort);
        self.mode = Mode::Results;
        // Lines 1160/1198/1250: the cost lands before anything can go wrong.
        self.state.savings -= info.cost();
        self.log.push(LogLine::head(format!(
            "YEAR {} — {} ({})",
            self.state.year, info.name, info.city
        )));

        let mut prices = fort_prices(
            fort,
            self.state.start_ermine_price,
            self.state.start_beaver_price,
            &mut self.rng,
        );
        let event = fort_event(fort, &mut self.rng);
        match (fort, event) {
            (Fort::Hochelaga, _) => {
                self.log.push(LogLine::line("YOU ARRIVED AT FORT HOCHELAGA."));
            }
            (Fort::Stadacona, Event::BeaverStolen) => {
                self.state.furs[Fur::Beaver as usize] = 0; // line 1216
                self.log.push(LogLine::alert(
                    "YOUR BEAVER WERE TOO HEAVY TO CARRY ACROSS THE PORTAGE.",
                ));
                self.log.push(LogLine::alert(
                    "YOU HAD TO LEAVE THE PELTS, BUT FOUND THEM STOLEN WHEN YOU RETURNED.",
                ));
            }
            (Fort::Stadacona, Event::Safe) => {
                self.log.push(LogLine::line("YOU ARRIVED SAFELY AT FORT STADACONA."));
            }
            (Fort::Stadacona, Event::AllFursLost) => {
                self.state.furs = [0; 4]; // line 1226 → GOSUB 1430
                self.log.push(LogLine::alert("YOUR CANOE UPSET IN THE LACHINE RAPIDS."));
                self.log.push(LogLine::alert("YOU LOST ALL YOUR FURS."));
            }
            (Fort::Stadacona, Event::FoxUncured) => {
                self.state.furs[Fur::Fox as usize] = 0; // line 1235
                self.log.push(LogLine::alert("YOUR FOX PELTS WERE NOT CURED PROPERLY."));
                self.log.push(LogLine::alert("NO ONE WILL BUY THEM."));
            }
            (Fort::NewYork, Event::Killed) => {
                // Lines 1281–1284: the journal ends here.
                self.log.push(LogLine::alert("YOU WERE ATTACKED BY A PARTY OF IROQUOIS."));
                self.log.push(LogLine::alert("ALL PEOPLE IN YOUR TRADING GROUP WERE KILLED."));
                self.outcome = Some(scoring::killed(&self.state));
                return;
            }
            (Fort::NewYork, Event::Safe) => {
                self.log.push(LogLine::line(
                    "YOU WERE LUCKY.  YOU ARRIVED SAFELY AT FORT NEW YORK.",
                ));
            }
            (Fort::NewYork, Event::AllFursLost) => {
                self.state.furs = [0; 4]; // line 1295 → GOSUB 1430
                self.log.push(LogLine::alert("YOU NARROWLY ESCAPED AN IROQUOIS RAIDING PARTY."));
                self.log.push(LogLine::alert("HOWEVER, YOU HAD TO LEAVE ALL YOUR FURS BEHIND."));
            }
            (Fort::NewYork, Event::FursDamaged) => {
                // Lines 1306–1307: halved raw, not re-rounded.
                prices[Fur::Beaver as usize] /= 2.0;
                prices[Fur::Mink as usize] /= 2.0;
                self.log.push(LogLine::alert("YOUR MINK AND BEAVER WERE DAMAGED ON YOUR TRIP."));
                self.log.push(LogLine::alert(
                    "YOU RECEIVE ONLY HALF THE CURRENT PRICE FOR THESE FURS.",
                ));
            }
            (fort, event) => unreachable!("event {event:?} cannot happen at {fort:?}"),
        }
        self.log.push(LogLine::line(format!(
            "SUPPLIES AT {} COST {}.",
            info.name,
            fmt_dollars(info.supplies)
        )));
        self.log.push(LogLine::line(format!(
            "YOUR TRAVEL EXPENSES WERE {}.",
            fmt_dollars(info.travel)
        )));

        // Line 1418: I = M1*F(1) + B1*F(2) + E1*F(3) + D1*F(4) + I.
        let mut proceeds = [0.0; 4];
        for (p, (price, count)) in
            proceeds.iter_mut().zip(prices.iter().zip(self.state.furs.iter()))
        {
            *p = price * *count as f64;
        }
        self.state.savings += proceeds.iter().sum::<f64>();
        self.state.sale_prices = prices;
        self.state.last_sale = Some(proceeds);
        // Lines 1412–1417 print in beaver, fox, ermine, mink order.
        for fur in [Fur::Beaver, Fur::Fox, Fur::Ermine, Fur::Mink] {
            self.log.push(LogLine::line(format!(
                "YOUR {} SOLD FOR {}",
                fur.name(),
                fmt_dollars(proceeds[fur as usize])
            )));
        }
        self.log.push(LogLine::line(format!(
            "YOU NOW HAVE {} INCLUDING YOUR PREVIOUS SAVINGS",
            fmt_dollars(self.state.savings)
        )));
    }

    // ----- end of the visit -----

    /// The run has been decided — the current playback is the last one.
    pub fn decided(&self) -> bool {
        self.outcome.is_some()
    }

    /// "DO YOU WANT TO TRADE FURS NEXT YEAR?" — YES (line 508). May end
    /// the run instead: bankruptcy is journaled and paced like any event.
    pub fn next_year(&mut self) {
        if self.mode != Mode::Results || self.decided() {
            return;
        }
        self.begin_year();
        if self.outcome.is_none() {
            self.mode = Mode::Allocate;
        }
    }

    /// Hang up the paddle: bank the savings and enter the ledger.
    pub fn retire(&mut self) {
        if self.mode != Mode::Results || self.decided() {
            return;
        }
        self.outcome = Some(scoring::retired(&self.state));
    }

    /// Flip to the reckoning screen once playback has finished.
    pub fn finish_visit(&mut self) {
        if self.outcome.is_some() {
            self.mode = Mode::GameOver;
        }
    }

    /// Clamp a deserialized ledger to sane ranges. Engine-driven play never
    /// leaves them, but a hand-edited or corrupted localStorage save is
    /// accepted as-is by serde and could otherwise wedge the arithmetic.
    pub fn sanitize(&mut self) {
        const CAP: f64 = 1.0e12;
        let st = &mut self.state;
        st.year = st.year.clamp(0, 1_000_000);
        if !st.savings.is_finite() {
            st.savings = 0.0;
        }
        st.savings = st.savings.clamp(-100_000.0, CAP);
        let mut remaining = FURS_PER_YEAR;
        for f in st.furs.iter_mut() {
            *f = (*f).clamp(0, remaining);
            remaining -= *f;
        }
        st.cursor = st.cursor.min(FUR_ORDER.len());
        for p in st
            .sale_prices
            .iter_mut()
            .chain([&mut st.start_ermine_price, &mut st.start_beaver_price])
        {
            if !p.is_finite() || !(0.0..=1_000.0).contains(p) {
                *p = 1.0;
            }
        }
        if let Some(sale) = st.last_sale.as_mut() {
            for v in sale.iter_mut() {
                if !v.is_finite() || *v < 0.0 {
                    *v = 0.0;
                }
            }
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new(retro_kit::rng::random_seed())
    }
}

#[cfg(test)]
mod tests {
    use super::scoring::Verdict;
    use super::state::FORT_ORDER;
    use super::*;

    const EPS: f64 = 1e-9;

    fn started(seed: u64) -> Game {
        let mut game = Game::new(seed);
        game.start();
        game
    }

    /// 40 mink, 60 beaver, 50 ermine, the rest (40) fox.
    const ALLOC: [i64; 4] = [40, 60, 50, 40];

    fn allocate_default(game: &mut Game) {
        for n in ALLOC {
            game.submit_fur(n);
        }
        game.finish_allocation();
    }

    /// A committed visit to `fort` with the default allocation.
    fn at_results(seed: u64, fort: Fort) -> Game {
        let mut game = started(seed);
        allocate_default(&mut game);
        game.choose_fort(fort);
        game.confirm_fort();
        game
    }

    fn log_text(game: &Game) -> String {
        game.log.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n")
    }

    fn assert_cents(p: f64) {
        assert!((p * 100.0 - (p * 100.0).round()).abs() < EPS, "{p} is not whole cents");
    }

    #[test]
    fn round2_matches_the_basic_idiom() {
        assert_eq!(round2(0.955), 0.96);
        assert_eq!(round2(0.954), 0.95);
        assert_eq!(round2(0.95), 0.95);
        assert_eq!(round2(1.0), 1.0);
        assert_eq!(round2(0.7), 0.7);
        assert_eq!(round2(1.349_99), 1.35);
    }

    #[test]
    fn start_prices_match_lines_261_262() {
        for seed in 0..200 {
            let mut rng = GameRng::from_seed(seed);
            let (ermine, beaver) = start_prices(&mut rng);
            assert!((0.95..=1.10).contains(&ermine), "ermine {ermine}");
            assert!((1.00..=1.25).contains(&beaver), "beaver {beaver}");
            assert_cents(ermine);
            assert_cents(beaver);
        }
    }

    #[test]
    fn hochelaga_prices_match_lines_1174_1177() {
        for seed in 0..200 {
            let mut rng = GameRng::from_seed(seed);
            let [mink, beaver, ermine, fox] = fort_prices(Fort::Hochelaga, 1.0, 1.1, &mut rng);
            assert!((0.70..=0.90).contains(&mink), "mink {mink}");
            assert!((0.75..=0.95).contains(&beaver), "beaver {beaver}");
            assert!((0.65..=0.85).contains(&ermine), "ermine {ermine}");
            assert!((0.80..=1.00).contains(&fox), "fox {fox}");
            for p in [mink, beaver, ermine, fox] {
                assert_cents(p);
            }
        }
    }

    #[test]
    fn stadacona_prices_match_lines_1205_1207_plus_the_fox_fix() {
        for seed in 0..200 {
            let mut rng = GameRng::from_seed(seed);
            let [mink, beaver, ermine, fox] = fort_prices(Fort::Stadacona, 1.0, 1.1, &mut rng);
            assert!((0.85..=1.15).contains(&mink), "mink {mink}");
            assert!((0.90..=1.10).contains(&beaver), "beaver {beaver}");
            assert!((0.80..=0.95).contains(&ermine), "ermine {ermine}");
            // The deviation: the original would leave fox stale (or $0.00).
            assert!((0.80..=1.00).contains(&fox), "fox {fox}");
            for p in [mink, beaver, ermine, fox] {
                assert_cents(p);
            }
        }
    }

    #[test]
    fn new_york_rolls_mink_and_fox_and_carries_the_start_prices() {
        for seed in 0..200 {
            let mut rng = GameRng::from_seed(seed);
            let (start_ermine, start_beaver) = start_prices(&mut rng);
            let [mink, beaver, ermine, fox] =
                fort_prices(Fort::NewYork, start_ermine, start_beaver, &mut rng);
            assert!((1.05..=1.20).contains(&mink), "mink {mink}");
            assert!((1.10..=1.35).contains(&fox), "fox {fox}");
            // Lines 1260–1263 never touch E1/B1: the year's high defaults.
            assert_eq!(ermine, start_ermine);
            assert_eq!(beaver, start_beaver);
            assert_cents(mink);
            assert_cents(fox);
        }
    }

    #[test]
    fn event_odds_match_the_p_ladder() {
        // P=INT(10*RND)+1: 1–2 / 3–6 / 7–8 / 9–10 → 20/40/20/20.
        for fort in [Fort::Stadacona, Fort::NewYork] {
            let mut rng = GameRng::from_seed(99);
            let mut counts = std::collections::HashMap::new();
            let n = 100_000;
            for _ in 0..n {
                *counts.entry(fort_event(fort, &mut rng)).or_insert(0) += 1;
            }
            let rate = |e: Event| *counts.get(&e).unwrap_or(&0) as f64 / n as f64;
            let (first, last) = match fort {
                Fort::Stadacona => (Event::BeaverStolen, Event::FoxUncured),
                Fort::NewYork => (Event::Killed, Event::FursDamaged),
                Fort::Hochelaga => unreachable!(),
            };
            assert!((0.19..0.21).contains(&rate(first)), "{fort:?} p<=2 {}", rate(first));
            assert!((0.39..0.41).contains(&rate(Event::Safe)), "{fort:?} safe");
            assert!((0.19..0.21).contains(&rate(Event::AllFursLost)), "{fort:?} lost");
            assert!((0.19..0.21).contains(&rate(last)), "{fort:?} p<=10 {}", rate(last));
        }
    }

    #[test]
    fn hochelaga_never_has_events_and_never_consumes_an_event_roll() {
        for seed in 0..50 {
            // Same seed, two streams: one plays Hochelaga, the other only
            // draws the four price rolls. They must stay in lockstep.
            let mut rng = GameRng::from_seed(seed);
            assert_eq!(fort_event(Fort::Hochelaga, &mut rng), Event::Safe);
            let game = at_results(seed, Fort::Hochelaga);
            assert_eq!(game.state.furs, ALLOC, "furs untouched");
            assert!(!game.decided());
            let mut reference = GameRng::from_seed(seed);
            let _ = start_prices(&mut reference); // begin_year's two rolls
            let expected = fort_prices(Fort::Hochelaga, 0.0, 0.0, &mut reference);
            assert_eq!(game.state.sale_prices, expected);
        }
    }

    #[test]
    fn the_sale_is_cost_then_price_times_count() {
        for seed in 0..100 {
            let game = at_results(seed, Fort::Hochelaga);
            let expected: f64 = game
                .state
                .sale_prices
                .iter()
                .zip(ALLOC)
                .map(|(p, n)| p * n as f64)
                .sum();
            assert!(
                (game.state.savings - (600.0 - 160.0 + expected)).abs() < EPS,
                "seed {seed}: savings {}",
                game.state.savings
            );
            assert_eq!(game.state.last_sale.unwrap()[0], game.state.sale_prices[0] * 40.0);
        }
    }

    #[test]
    fn stadacona_events_zero_the_right_pelts_and_still_charge_140() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..300 {
            let game = at_results(seed, Fort::Stadacona);
            let text = log_text(&game);
            let sale = game.state.last_sale.unwrap();
            if text.contains("STOLEN") {
                seen.insert("stolen");
                assert_eq!(game.state.furs[Fur::Beaver as usize], 0);
                assert_eq!(sale[Fur::Beaver as usize], 0.0);
                assert!(sale[Fur::Mink as usize] > 0.0);
            } else if text.contains("CANOE UPSET") {
                seen.insert("upset");
                assert_eq!(game.state.furs, [0; 4]);
                assert!(sale.iter().all(|p| *p == 0.0));
                // Lines 1226–1232: you sell nothing yet pay in full.
                assert!((game.state.savings - (600.0 - 140.0)).abs() < EPS);
            } else if text.contains("NOT CURED") {
                seen.insert("uncured");
                assert_eq!(game.state.furs[Fur::Fox as usize], 0);
                assert_eq!(sale[Fur::Fox as usize], 0.0);
                assert!(sale[Fur::Beaver as usize] > 0.0);
            } else {
                seen.insert("safe");
                assert!(text.contains("ARRIVED SAFELY"));
                assert!(sale.iter().all(|p| *p > 0.0));
            }
            assert!(!game.decided(), "no Stadacona event ends the game");
        }
        assert_eq!(seen.len(), 4, "all four branches exercised: {seen:?}");
    }

    #[test]
    fn new_york_events_kill_loot_or_halve_prices() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..300 {
            let game = at_results(seed, Fort::NewYork);
            let text = log_text(&game);
            if text.contains("ATTACKED") {
                seen.insert("killed");
                let end = game.outcome.as_ref().expect("the ambush ends the run");
                assert_eq!(end.verdict, Verdict::Killed);
                assert_eq!(end.years, 1);
                // Paid for the trip, sold nothing, died.
                assert!((game.state.savings - (600.0 - 105.0)).abs() < EPS);
                assert!(game.state.last_sale.is_none());
                assert_eq!(game.mode, Mode::Results, "mode flips only in finish_visit");
            } else if text.contains("NARROWLY ESCAPED") {
                seen.insert("escaped");
                assert_eq!(game.state.furs, [0; 4]);
                assert!((game.state.savings - (600.0 - 105.0)).abs() < EPS);
            } else if text.contains("DAMAGED") {
                seen.insert("damaged");
                // Lines 1306–1307: B1=B1/2, M1=M1/2 — raw halving.
                let prices = game.state.sale_prices;
                assert!((1.05..=1.20).contains(&(prices[Fur::Mink as usize] * 2.0)));
                assert!(
                    (prices[Fur::Beaver as usize] * 2.0 - game.state.start_beaver_price).abs()
                        < EPS
                );
                assert_eq!(prices[Fur::Ermine as usize], game.state.start_ermine_price);
            } else {
                seen.insert("safe");
                assert!(text.contains("YOU WERE LUCKY"));
                let prices = game.state.sale_prices;
                assert_eq!(prices[Fur::Beaver as usize], game.state.start_beaver_price);
                assert_eq!(prices[Fur::Ermine as usize], game.state.start_ermine_price);
            }
        }
        assert_eq!(seen.len(), 4, "all four branches exercised: {seen:?}");
    }

    #[test]
    fn allocation_clamps_to_the_190_furs() {
        let mut game = started(1);
        assert_eq!(game.remaining(), 190);
        assert_eq!(game.current_fur(), Some(Fur::Mink));
        game.submit_fur(1_000_000);
        assert_eq!(game.state.furs[0], 190, "clamped to the bale");
        assert_eq!(game.remaining(), 0);
        game.submit_fur(50);
        assert_eq!(game.state.furs[1], 0, "nothing left to assign");
        game.submit_fur(-5);
        assert_eq!(game.state.furs[2], 0, "negatives clamp to zero");
        assert_eq!(game.current_fur(), Some(Fur::Fox));
        game.finish_allocation();
        assert_eq!(game.mode, Mode::Allocate, "all four prompts must be answered");
        game.submit_fur(0);
        assert_eq!(game.current_fur(), None);
        game.finish_allocation();
        assert_eq!(game.mode, Mode::FortSelect);
    }

    #[test]
    fn taking_fewer_than_190_furs_is_allowed() {
        let mut game = started(1);
        for _ in 0..4 {
            game.submit_fur(10);
        }
        game.finish_allocation();
        assert_eq!(game.mode, Mode::FortSelect);
        assert_eq!(game.state.allocated(), 40);
    }

    #[test]
    fn redo_resets_the_distribution() {
        let mut game = started(1);
        game.submit_fur(100);
        game.submit_fur(90);
        game.redo_allocation();
        assert_eq!(game.state.furs, [0; 4]);
        assert_eq!(game.remaining(), 190);
        assert_eq!(game.current_fur(), Some(Fur::Mink));
    }

    #[test]
    fn changing_your_mind_about_the_fort_is_free() {
        let mut game = started(7);
        allocate_default(&mut game);
        let snapshot = game.clone();
        game.choose_fort(Fort::NewYork);
        assert_eq!(game.mode, Mode::FortConfirm);
        assert_eq!(game.state.fort, Some(Fort::NewYork));
        game.change_fort();
        assert_eq!(game, snapshot, "no cost, no RNG, no state until confirm");
    }

    #[test]
    fn bankruptcy_ends_the_run_at_the_next_year() {
        let mut game = at_results(2, Fort::Hochelaga);
        game.state.savings = MIN_EXPEDITION_COST - 0.01;
        game.next_year();
        let end = game.outcome.as_ref().expect("bankrupt");
        assert_eq!(end.verdict, Verdict::Bankrupt);
        assert_eq!(end.years, 1, "only completed years count");
        assert_eq!(game.state.year, 1, "the new year never started");
        assert_eq!(game.mode, Mode::Results, "the bad news paces in the journal");
        assert!(log_text(&game).contains("TRADING DAYS ARE OVER"));
        game.finish_visit();
        assert_eq!(game.mode, Mode::GameOver);
    }

    #[test]
    fn exactly_105_still_buys_an_expedition() {
        let mut game = at_results(2, Fort::Hochelaga);
        game.state.savings = MIN_EXPEDITION_COST;
        game.next_year();
        assert!(!game.decided());
        assert_eq!(game.state.year, 2);
        assert_eq!(game.mode, Mode::Allocate);
        assert_eq!(game.remaining(), 190, "fresh furs each year");
    }

    #[test]
    fn retiring_banks_the_savings() {
        let mut game = at_results(3, Fort::Hochelaga);
        let savings = game.state.savings;
        game.retire();
        let end = game.outcome.as_ref().expect("retired");
        assert_eq!(end.verdict, Verdict::Retired);
        assert_eq!(end.years, 1);
        assert_eq!(end.score, savings.floor() as i64);
        assert!(!end.recorded);
        game.finish_visit();
        assert_eq!(game.mode, Mode::GameOver);
    }

    #[test]
    fn inputs_after_the_run_is_decided_are_inert() {
        let mut game = at_results(2, Fort::Hochelaga);
        game.state.savings = 50.0;
        game.next_year(); // bankrupt
        assert!(game.decided());
        let snapshot = game.clone();
        game.submit_fur(10);
        game.choose_fort(Fort::NewYork);
        game.confirm_fort();
        game.next_year();
        game.retire();
        assert_eq!(game, snapshot);
    }

    #[test]
    fn save_round_trip_preserves_the_game() {
        let mut game = started(5);
        allocate_default(&mut game);
        game.choose_fort(Fort::Stadacona);
        let json = serde_json::to_string(&game).expect("serialize");
        let back: Game = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(game, back);
    }

    #[test]
    fn corrupt_saves_are_sanitized_and_safe() {
        let mut game = started(1);
        game.state.savings = f64::NAN;
        game.state.furs = [500, -3, 100, 100];
        game.state.cursor = 99;
        game.state.start_ermine_price = f64::INFINITY;
        game.state.sale_prices = [-1.0, f64::NAN, 2.0, 1e9];
        game.sanitize();
        assert_eq!(game.state.savings, 0.0);
        assert_eq!(game.state.furs, [190, 0, 0, 0], "total clamped back to the bale");
        assert_eq!(game.state.cursor, 4);
        assert_eq!(game.state.start_ermine_price, 1.0);
        assert_eq!(game.state.sale_prices, [1.0, 1.0, 2.0, 1.0]);
        // ...and a visit still resolves without panicking.
        game.mode = Mode::FortConfirm;
        game.state.fort = Some(Fort::Stadacona);
        game.confirm_fort();
        assert!(game.state.savings.is_finite());
    }

    /// Play whole runs through the public API across many seeds: allocate
    /// everything, rotate forts, retire after year 8. Assert the invariants
    /// hold and every run actually ends.
    #[test]
    fn scripted_runs_stay_sane_across_seeds() {
        let mut verdicts = std::collections::HashMap::new();
        for seed in 0..200 {
            let mut game = started(seed);
            let mut guard = 0;
            while !game.decided() {
                guard += 1;
                assert!(guard < 100, "seed {seed}: wedged");
                let quarter = game.remaining() / 4;
                for _ in 0..3 {
                    game.submit_fur(quarter);
                }
                game.submit_fur(game.remaining());
                assert_eq!(game.state.allocated(), FURS_PER_YEAR);
                game.finish_allocation();
                assert_eq!(game.mode, Mode::FortSelect, "seed {seed}");
                let fort = FORT_ORDER[(game.state.year as usize - 1) % 3];
                game.choose_fort(fort);
                game.confirm_fort();
                assert!(game.state.savings.is_finite());
                assert!(game.state.savings > -200.0, "seed {seed}: deep in debt");
                assert!(game.state.allocated() <= FURS_PER_YEAR);
                assert_eq!(game.mode, Mode::Results, "seed {seed}");
                if game.decided() {
                    break;
                }
                if game.state.year >= 8 {
                    game.retire();
                } else {
                    game.next_year();
                }
            }
            let end = game.outcome.as_ref().unwrap();
            *verdicts.entry(end.verdict).or_insert(0) += 1;
            assert_eq!(end.score, end.savings.max(0.0).floor() as i64, "seed {seed}");
            game.finish_visit();
            assert_eq!(game.mode, Mode::GameOver);
        }
        // Rotating through New York ~3 times a run, both endings appear.
        assert!(verdicts[&Verdict::Killed] > 0, "{verdicts:?}");
        assert!(verdicts[&Verdict::Retired] > 0, "{verdicts:?}");
    }
}
