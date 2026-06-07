//! Core state types shared by both mission variants. Field names mirror the
//! BASIC variables (see /tmp source or the plan spec) for auditability.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionKind {
    /// Jim Storer's classic: 120 mi up, burn 0–200 lb/s every 10 seconds.
    Lunar,
    /// Eric Peters' ROCKET: 1000 ft up, burn 0–30 fuel units each second.
    Rocket,
}

impl MissionKind {
    pub fn title(&self) -> &'static str {
        match self {
            MissionKind::Lunar => "LUNAR",
            MissionKind::Rocket => "ROCKET",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Splash,
    MissionSelect,
    Flight,
    GameOver,
}

/// LUNAR craft state (lunar.bas line 140): `A=120:V=1:M=33000:N=16500`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LunarState {
    /// Altitude in miles, up positive.
    pub a: f64,
    /// Velocity in miles/sec, down positive.
    pub v: f64,
    /// Total mass (capsule + fuel) in pounds.
    pub m: f64,
    /// Elapsed seconds.
    pub l: f64,
}

impl LunarState {
    pub fn new() -> Self {
        Self { a: 120.0, v: 1.0, m: 33_000.0, l: 0.0 }
    }
}

/// ROCKET craft state (rocket.bas line 455): `T=0:H=1000:V=50:F=150`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RocketState {
    /// Elapsed seconds.
    pub t: f64,
    /// Height in feet, up positive.
    pub h: f64,
    /// Velocity in feet/sec, down positive.
    pub v: f64,
    /// Fuel units remaining.
    pub f: f64,
}

impl RocketState {
    pub fn new() -> Self {
        Self { t: 0.0, h: 1000.0, v: 50.0, f: 150.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Flight {
    Lunar(LunarState),
    Rocket(RocketState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Landing {
    Perfect,
    Good,
    Safe,
    Damaged,
    Crash,
}

impl Landing {
    /// Worth recording on the board: walked away with craft intact.
    pub fn landed_ok(&self) -> bool {
        matches!(self, Landing::Perfect | Landing::Good | Landing::Safe)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Landing::Perfect => "Perfect landing",
            Landing::Good => "Good landing",
            Landing::Safe => "Safe landing",
            Landing::Damaged => "Craft damaged",
            Landing::Crash => "Crashed",
        }
    }
}

/// The final reckoning, built once on contact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGame {
    pub mission: MissionKind,
    pub quality: Landing,
    pub headline: String,
    pub detail: String,
    /// Display string: "x.x MPH" (LUNAR) or "x.x FT/S" (ROCKET).
    pub impact: String,
    pub elapsed: f64,
    pub fuel_left: f64,
    pub score: i64,
    /// Guards the once-only high-score record in the autosave effect.
    pub recorded: bool,
}

/// One line of the mission log: a telemetry row or a banner message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LogLine {
    Row(TurnRow),
    Banner(String),
}

/// Pre-formatted telemetry, mirroring the original report tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnRow {
    pub sec: String,
    pub alt: String,
    pub vel: String,
    pub fuel: String,
    pub burn: String,
}

pub use retro_kit::format::fmt_num;

/// LUNAR altitude the way line 150 reports it: `INT(A)` miles + leftover feet.
pub fn fmt_lunar_alt(a: f64) -> String {
    let miles = a.floor();
    let feet = (5280.0 * (a - miles)).floor();
    format!("{miles:.0} mi {feet:.0} ft")
}
