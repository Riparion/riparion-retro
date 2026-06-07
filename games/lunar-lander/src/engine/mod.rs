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
    pub mission: MissionKind,
    pub flight: Flight,
    /// The mission log (telemetry rows + banners), grown by the UI as it
    /// paces through what `take_turn` returns.
    pub log: Vec<LogLine>,
    pub outcome: Option<EndGame>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            mode: Mode::Splash,
            mission: MissionKind::Lunar,
            flight: Flight::Lunar(LunarState::new()),
            log: Vec::new(),
            outcome: None,
        }
    }

    /// Begin a descent: fresh craft, fresh log seeded with the first report.
    pub fn start(&mut self, mission: MissionKind) {
        self.mission = mission;
        self.outcome = None;
        self.flight = match mission {
            MissionKind::Lunar => Flight::Lunar(LunarState::new()),
            MissionKind::Rocket => Flight::Rocket(RocketState::new()),
        };
        self.log = vec![LogLine::Row(match &self.flight {
            Flight::Lunar(st) => lunar::report_row(st, "—"),
            Flight::Rocket(st) => rocket::report_row(st, "—"),
        })];
        self.mode = Mode::Flight;
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

    pub fn push_log(&mut self, line: LogLine) {
        self.log.push(line);
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
        match self.mission {
            MissionKind::Lunar => lunar::BURN_MAX,
            MissionKind::Rocket => rocket::BURN_MAX,
        }
    }

    pub fn fuel_remaining(&self) -> f64 {
        match &self.flight {
            Flight::Lunar(st) => lunar::fuel(st),
            Flight::Rocket(st) => st.f,
        }
    }

    /// Altitude as a fraction of the mission's starting height, for the
    /// descent strip. Clamped to 0..=1.
    pub fn altitude_frac(&self) -> f64 {
        let frac = match &self.flight {
            Flight::Lunar(st) => st.a / lunar::START_ALTITUDE,
            Flight::Rocket(st) => st.h / rocket::START_HEIGHT,
        };
        frac.clamp(0.0, 1.0)
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
        assert!(game.fuel_remaining() >= -1e-9, "fuel went negative");
        let (alt, vel, time) = match &game.flight {
            Flight::Lunar(st) => (st.a, st.v, st.l),
            Flight::Rocket(st) => (st.h, st.v, st.t),
        };
        for x in [alt, vel, time] {
            assert!(x.is_finite(), "non-finite state");
        }
    }

    /// Play a whole mission through the public API and check invariants.
    fn play(mission: MissionKind, burns: impl Fn(u32) -> i64) -> Game {
        let mut game = Game::new();
        game.start(mission);
        let mut last_time = -1.0;
        for turn in 0..300 {
            let lines = game.take_turn(burns(turn));
            for line in lines {
                game.push_log(line);
            }
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
        for line in game.take_turn(50) {
            game.push_log(line);
        }
        let json = serde_json::to_string(&game).expect("serialize");
        let back: Game = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(game, back);
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
