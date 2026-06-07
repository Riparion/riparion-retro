//! Pure game engine: no dioxus, no web — fully testable on the host. Both
//! mission variants are deterministic ports of the 1978 listings; there is
//! deliberately no RNG anywhere in this crate.

pub mod lunar;
pub mod rocket;
pub mod scoring;
pub mod state;

use serde::{Deserialize, Serialize};

use state::{EndGame, Flight, LogLine, LunarState, MissionKind, Mode, RocketState};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub mode: Mode,
    pub flight: Flight,
    /// The mission log (telemetry rows + banners), committed by the UI once
    /// it has paced through what `take_turn` returned.
    pub log: Vec<LogLine>,
    pub outcome: Option<EndGame>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            mode: Mode::Splash,
            flight: Flight::Lunar(LunarState::new()),
            log: Vec::new(),
            outcome: None,
        }
    }

    /// Which mission is being flown — derived from the flight state so the
    /// two can never disagree.
    pub fn mission(&self) -> MissionKind {
        match self.flight {
            Flight::Lunar(_) => MissionKind::Lunar,
            Flight::Rocket(_) => MissionKind::Rocket,
        }
    }

    /// Begin a descent: fresh craft, fresh log seeded with the first report.
    pub fn start(&mut self, mission: MissionKind) {
        self.outcome = None;
        self.flight = match mission {
            MissionKind::Lunar => Flight::Lunar(LunarState::new()),
            MissionKind::Rocket => Flight::Rocket(RocketState::new()),
        };
        self.log = vec![LogLine::Row(self.current_report())];
        self.mode = Mode::Flight;
    }

    /// A telemetry row for the live craft state (the fallback when the log
    /// holds no row yet).
    pub fn current_report(&self) -> state::TurnRow {
        match &self.flight {
            Flight::Lunar(st) => lunar::report_row(st, "—"),
            Flight::Rocket(st) => rocket::report_row(st, "—"),
        }
    }

    /// Resolve one burn input. Returns the new log lines for paced playback
    /// (the UI appends them via [`Game::push_log`]); on contact the outcome is
    /// stored here and the mode flips only in [`Game::finish_flight`], so the
    /// screen never changes mid-playback.
    pub fn take_turn(&mut self, burn: i64) -> Vec<LogLine> {
        if self.outcome.is_some() {
            return Vec::new();
        }
        match &mut self.flight {
            Flight::Lunar(st) => {
                let out = lunar::step(st, burn);
                if let Some(mph) = out.impact_mph {
                    self.outcome = Some(scoring::lunar_end_game(mph, st.l, lunar::fuel(st)));
                }
                out.lines
            }
            Flight::Rocket(st) => {
                let out = rocket::step(st, burn);
                if let Some(c) = out.contact {
                    self.outcome =
                        Some(scoring::rocket_end_game(c.landing_v, c.touchdown_at, st.f));
                }
                out.lines
            }
        }
    }

    /// Commit a turn's paced lines to the persistent log in one write.
    pub fn extend_log(&mut self, lines: impl IntoIterator<Item = LogLine>) {
        self.log.extend(lines);
    }

    /// Contact made — the current turn's playback is the last one.
    pub fn decided(&self) -> bool {
        self.outcome.is_some()
    }

    /// Flip to the reckoning screen once playback has finished.
    pub fn finish_flight(&mut self) {
        if self.outcome.is_some() {
            self.mode = Mode::GameOver;
        }
    }

    pub fn burn_max(&self) -> i64 {
        match self.mission() {
            MissionKind::Lunar => lunar::BURN_MAX,
            MissionKind::Rocket => rocket::BURN_MAX,
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use state::Landing;

    fn assert_sane(game: &Game) {
        let (alt, vel, time, fuel) = match &game.flight {
            Flight::Lunar(st) => (st.a, st.v, st.l, lunar::fuel(st)),
            Flight::Rocket(st) => (st.h, st.v, st.t, st.f),
        };
        assert!(fuel >= -1e-9, "fuel went negative");
        for x in [alt, vel, time] {
            assert!(x.is_finite(), "non-finite state");
        }
        let row = game.current_report();
        assert!((0.0..=1.0).contains(&row.alt_frac), "alt_frac out of range");
    }

    /// Play a whole mission through the public API and check invariants.
    fn play(mission: MissionKind, burns: impl Fn(u32) -> i64) -> Game {
        let mut game = Game::new();
        game.start(mission);
        let mut last_time = -1.0;
        for turn in 0..300 {
            let lines = game.take_turn(burns(turn));
            game.extend_log(lines);
            assert_sane(&game);
            let time = match &game.flight {
                Flight::Lunar(st) => st.l,
                Flight::Rocket(st) => st.t,
            };
            assert!(time > last_time, "time must advance");
            last_time = time;
            if game.decided() {
                game.finish_flight();
                assert_eq!(game.mode, Mode::GameOver);
                return game;
            }
        }
        panic!("mission never reached the surface");
    }

    #[test]
    fn lunar_free_fall_game_crashes() {
        let game = play(MissionKind::Lunar, |_| 0);
        let end = game.outcome.expect("ended");
        assert_eq!(end.quality, Landing::Crash);
        assert_eq!(end.score, 0);
    }

    #[test]
    fn lunar_classic_strategy_lands() {
        // The well-known opening: free fall, then hard braking. Not asserted
        // to land softly — only that the game resolves and stays sane.
        let game = play(MissionKind::Lunar, |t| if t < 7 { 0 } else { 170 });
        assert!(game.outcome.is_some());
    }

    #[test]
    fn rocket_steady_burn_game_resolves() {
        let game = play(MissionKind::Rocket, |_| 10);
        assert!(game.outcome.is_some());
    }

    #[test]
    fn rocket_perfect_landing_recordable() {
        // Sculpted final approach (see rocket.rs tests for the derivation).
        let mut game = Game::new();
        game.start(MissionKind::Rocket);
        game.flight = Flight::Rocket(RocketState { t: 12.0, h: 5.0, v: 10.0, f: 20.0 });
        let lines = game.take_turn(15);
        assert!(!lines.is_empty());
        let end = game.outcome.expect("contact");
        assert_eq!(end.quality, Landing::Perfect);
        assert!(end.quality.landed_ok());
        assert_eq!(end.score, scoring::score(MissionKind::Rocket, Landing::Perfect, 5.0));
    }

    #[test]
    fn save_round_trip_mid_flight() {
        let mut game = Game::new();
        game.start(MissionKind::Lunar);
        let lines = game.take_turn(50);
        game.extend_log(lines);
        let json = serde_json::to_string(&game).expect("serialize");
        let back: Game = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(game, back);
    }

    /// A save written during contact playback has outcome set but mode still
    /// Flight; loading must heal it to GameOver (`load_or_new` calls
    /// `finish_flight`), otherwise the flight screen soft-locks — no input
    /// dispatches a turn once `decided()`, so nothing else flips the mode.
    #[test]
    fn decided_flight_save_heals_to_game_over() {
        let mut game = Game::new();
        game.start(MissionKind::Rocket);
        game.flight = Flight::Rocket(RocketState { t: 0.0, h: 1.0, v: 50.0, f: 0.0 });
        game.take_turn(0);
        assert!(game.decided());
        assert_eq!(game.mode, Mode::Flight, "mode flips only in finish_flight");
        let json = serde_json::to_string(&game).expect("serialize");
        let mut back: Game = serde_json::from_str(&json).expect("deserialize");
        back.finish_flight(); // what load_or_new does on every load
        assert_eq!(back.mode, Mode::GameOver);
        // ...and an undecided save is left alone.
        let mut fresh = Game::new();
        fresh.start(MissionKind::Lunar);
        fresh.finish_flight();
        assert_eq!(fresh.mode, Mode::Flight);
    }

    #[test]
    fn take_turn_after_contact_is_inert() {
        let mut game = Game::new();
        game.start(MissionKind::Rocket);
        game.flight = Flight::Rocket(RocketState { t: 0.0, h: 1.0, v: 50.0, f: 0.0 });
        game.take_turn(0);
        assert!(game.decided());
        let snapshot = game.clone();
        assert!(game.take_turn(30).is_empty());
        assert_eq!(game, snapshot);
    }
}
