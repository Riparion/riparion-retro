//! Autonomous players. A [`Policy`] looks at a `Game` (and, when one exists, the
//! shared [`Market`]) and returns the next [`Action`]; [`run_to_end`] drives a
//! game to its ending by stepping a policy. This is how bots play the exact same
//! engine a human plays — through the same [`Action`] vocabulary.
//!
//! The minigame/interaction resolutions reuse the *same* outcome bands the
//! golden-trace feed in `tests.rs` exercises (see [`minigame_action`]), so bots
//! stress precisely the branches the behavior oracle pins.

use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::interaction::{Interaction, Response};
use crate::market::Market;
use crate::market_link::Side;
use crate::rng::GameRng;
use crate::state::{self, BoatKind, Mode, Pace, NUM_GOODS};
use crate::Game;

/// A bot's trading temperament — surfaced to players as flavor in trader gossip
/// (see [`crate::gossip`]). Maps from the concrete policy a bot runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Persona {
    /// Loads up cheap and presses downstream for the margin.
    Greedy,
    /// Convoys, takes the safe line, rests up.
    Cautious,
    /// Trades and gambles on a whim.
    Reckless,
}

impl Persona {
    /// Stable key for looking up the persona's epithet in the scenario flavor.
    pub fn key(self) -> &'static str {
        match self {
            Persona::Greedy => "greedy",
            Persona::Cautious => "cautious",
            Persona::Reckless => "reckless",
        }
    }
}

/// An autonomous decision-maker. Implementors return one [`Action`] per call;
/// the host applies it and calls again. `Send` so a [`World`](crate::world) of
/// bots can run inside the server's async task.
pub trait Policy: Send {
    /// Decide the next action given the current game and the shared market (if
    /// the game is linked to one).
    fn decide(&mut self, game: &Game, market: Option<&Market>) -> Action;

    /// This policy's trading temperament, surfaced in trader gossip.
    fn persona(&self) -> Persona;

    /// Reset any per-journey state, so the same policy can drive a fresh journey
    /// (used by the persistent server when it respawns a finished bot). Stateless
    /// policies need not override this.
    fn reset(&mut self) {}
}

/// If the game is paused on an interaction prompt or a minigame, synthesize a
/// resolution for it. `variant` selects which outcome band to play — the bands
/// are exactly those in the golden-trace feed, so `variant` lets a policy bias
/// toward success (low remainders) or fuzz across all branches. Returns `None`
/// for hub modes (where a real decision is needed).
pub fn minigame_action(game: &Game, variant: u64) -> Option<Action> {
    Some(match game.mode {
        Mode::Interaction => {
            let resp = match game.pending.front() {
                Some(Interaction::CrewLeaves { .. }) | Some(Interaction::FerryToll { .. }) => {
                    if variant % 2 == 0 {
                        Response::Yes
                    } else {
                        Response::No
                    }
                }
                _ => Response::Acknowledge,
            };
            Action::Respond(resp)
        }
        Mode::Steady => {
            let (steady, accuracy) = match variant % 5 {
                0 => (true, 0.95),
                1 => (false, 0.55),
                2 => (true, 0.80),
                3 => (false, 0.30),
                _ => (false, 0.12),
            };
            Action::ResolveSteady { steady, accuracy }
        }
        Mode::Quick => {
            let (reaction_secs, hit) = match variant % 4 {
                0 => (0.5, true),
                1 => (1.4, true),
                2 => (2.0, false),
                _ => (0.8, true),
            };
            Action::ResolveQuick { reaction_secs, hit }
        }
        Mode::Crowd => {
            let (cleared, accuracy) = match variant % 3 {
                0 => (true, 0.9),
                1 => (false, 0.35),
                _ => (true, 0.7),
            };
            Action::ResolveCrowd { cleared, accuracy }
        }
        Mode::Timing => {
            let (hit, accuracy) = match variant % 4 {
                0 => (true, 0.9),
                1 => (false, 0.5),
                2 => (false, 0.1),
                _ => (true, 0.6),
            };
            Action::ResolveTiming { hit, accuracy }
        }
        Mode::Sequence => {
            let (correct_prefix, length, perfect) = match variant % 3 {
                0 => (4, 4, true),
                1 => (2, 4, false),
                _ => (3, 4, false),
            };
            Action::ResolveSequence {
                correct_prefix,
                length,
                perfect,
            }
        }
        Mode::Brigade => {
            let (contained, leaked, capacity) = match variant % 3 {
                0 => (true, 0, 9),
                1 => (false, 5, 9),
                _ => (false, 2, 9),
            };
            Action::ResolveBrigade {
                contained,
                leaked,
                capacity,
            }
        }
        Mode::Heave => {
            let (opened, slips, grip_left) = match variant % 4 {
                0 => (true, 0, 0.60),
                1 => (false, 2, 0.00),
                2 => (true, 1, 0.30),
                _ => (false, 1, 0.15),
            };
            Action::ResolveHeave {
                opened,
                slips,
                grip_left,
            }
        }
        Mode::HotCold => {
            let (found, probes_used, budget) = match variant % 4 {
                0 => (true, 2, 7),
                1 => (false, 7, 7),
                2 => (true, 6, 7),
                _ => (true, 4, 7),
            };
            Action::ResolveHotCold {
                found,
                probes_used,
                budget,
            }
        }
        Mode::Hunter => {
            let (hit, shots_fired) = match variant % 4 {
                0 => (true, 1),
                1 => (false, 3),
                2 => (true, 3),
                _ => (true, 2),
            };
            Action::ResolveHunter { hit, shots_fired }
        }
        _ => return None,
    })
}

/// Step a policy once: decide and apply a single action.
pub fn step(game: &mut Game, policy: &mut dyn Policy, market: Option<&Market>) {
    let action = policy.decide(game, market);
    game.apply(action);
}

/// Drive a standalone game (no shared market) to its ending, returning the
/// outcome. `max_steps` guards against a policy that never terminates.
pub fn run_to_end(game: &mut Game, policy: &mut dyn Policy, max_steps: u64) -> Option<state::EndGame> {
    let mut steps = 0;
    while game.outcome.is_none() && steps < max_steps {
        step(game, policy, None);
        steps += 1;
    }
    game.outcome.clone()
}

// ===== Stock strategies =====

/// The good a buyer should load here: the cheapest affordable unit, which favors
/// goods the town produces (they carry the supply discount). Returns `None` when
/// nothing is affordable or the hold is full.
fn best_buy_good(game: &Game) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for g in 0..NUM_GOODS {
        if game.max_buy(g) == 0 {
            continue;
        }
        let price = game.buy_price(g);
        match best {
            Some((_, bp)) if bp <= price => {}
            _ => best = Some((g, price)),
        }
    }
    best.map(|(g, _)| g)
}

/// The first good currently in the hold, if any.
fn first_held(game: &Game) -> Option<usize> {
    (0..NUM_GOODS).find(|&g| game.state.hold[g] > 0)
}

/// A profit-seeking trader: load up cheap, sell down the river, sell out and walk
/// home from Natchez. Plays minigames to win.
pub struct GreedyProfit {
    /// The river town index at which we've already restocked, so we don't sell
    /// the cargo we just bought there. Towns are visited once, downstream, so a
    /// single marker suffices.
    restocked_at: Option<usize>,
}

impl Default for GreedyProfit {
    fn default() -> Self {
        Self { restocked_at: None }
    }
}

impl Policy for GreedyProfit {
    fn persona(&self) -> Persona {
        Persona::Greedy
    }

    fn decide(&mut self, game: &Game, _market: Option<&Market>) -> Action {
        if let Some(a) = minigame_action(game, 0) {
            return a; // variant 0 → the success band of every minigame
        }
        match game.mode {
            Mode::Pittsburgh => {
                if !game.has_boat() {
                    return Action::Build {
                        kind: BoatKind::Flatboat,
                        crew: 3,
                    };
                }
                match best_buy_good(game) {
                    Some(g) => Action::Trade {
                        good: g,
                        side: Side::Buy,
                        qty: game.max_buy(g),
                    },
                    None => Action::Depart,
                }
            }
            Mode::Town | Mode::Trade { .. } => {
                let town = game.state.town;
                if self.restocked_at == Some(town) {
                    // Already reloaded here — load any remaining capacity, else go.
                    return match best_buy_good(game) {
                        Some(g) => Action::Trade {
                            good: g,
                            side: Side::Buy,
                            qty: game.max_buy(g),
                        },
                        None => Action::Depart,
                    };
                }
                // Sell everything we arrived with, one good at a time.
                if let Some(g) = first_held(game) {
                    return Action::Trade {
                        good: g,
                        side: Side::Sell,
                        qty: game.state.hold[g],
                    };
                }
                // Hold empty: mark restocked and start loading for the next leg.
                self.restocked_at = Some(town);
                match best_buy_good(game) {
                    Some(g) => Action::Trade {
                        good: g,
                        side: Side::Buy,
                        qty: game.max_buy(g),
                    },
                    None => Action::Depart,
                }
            }
            Mode::Natchez => {
                if let Some(g) = first_held(game) {
                    Action::Trade {
                        good: g,
                        side: Side::Sell,
                        qty: game.state.hold[g],
                    }
                } else if game.has_boat() {
                    Action::SellBoat
                } else {
                    Action::SetOutOnTrace
                }
            }
            Mode::Falls => Action::FallsPilot(8.0),
            Mode::GrandTower => Action::GrandTowerTreat(2.0),
            Mode::CaveInRock => Action::CaveHire(5.0),
            Mode::TraceHub => {
                if game.state.pace != Pace::Hard {
                    Action::SetPace(Pace::Hard)
                } else {
                    Action::TravelDay
                }
            }
            Mode::Stand => Action::LeaveStand,
            Mode::Moneylender => Action::Depart,
            _ => Action::Noop,
        }
    }

    fn reset(&mut self) {
        self.restocked_at = None;
    }
}

/// A risk-averse trader: convoy on, steady pace, rest when provisions run low,
/// pay tolls and keep the crew, and play minigames safe.
pub struct Cautious;

impl Policy for Cautious {
    fn persona(&self) -> Persona {
        Persona::Cautious
    }

    fn decide(&mut self, game: &Game, _market: Option<&Market>) -> Action {
        if let Some(a) = minigame_action(game, 0) {
            return a;
        }
        match game.mode {
            Mode::Pittsburgh => {
                if !game.has_boat() {
                    return Action::Build {
                        kind: BoatKind::Flatboat,
                        crew: 3,
                    };
                }
                // Load only half of capacity-per-good for a lighter, safer boat.
                match best_buy_good(game) {
                    Some(g) => Action::Trade {
                        good: g,
                        side: Side::Buy,
                        qty: (game.max_buy(g) / 2).max(1),
                    },
                    None => Action::Depart,
                }
            }
            Mode::Town | Mode::Trade { .. } => {
                if !game.state.river_convoy {
                    return Action::SetRiverConvoy(true);
                }
                if let Some(g) = first_held(game) {
                    Action::Trade {
                        good: g,
                        side: Side::Sell,
                        qty: game.state.hold[g],
                    }
                } else {
                    Action::Depart
                }
            }
            Mode::Natchez => {
                if let Some(g) = first_held(game) {
                    Action::Trade {
                        good: g,
                        side: Side::Sell,
                        qty: game.state.hold[g],
                    }
                } else if game.has_boat() {
                    Action::SellBoat
                } else {
                    Action::SetOutOnTrace
                }
            }
            Mode::Falls => Action::FallsPilot(8.0),
            Mode::GrandTower => Action::GrandTowerTreat(2.0),
            Mode::CaveInRock => Action::CaveHire(5.0),
            Mode::TraceHub => {
                if game.state.pace != Pace::Steady {
                    Action::SetPace(Pace::Steady)
                } else if !game.state.grouped {
                    Action::SetGrouped(true)
                } else {
                    Action::TravelDay
                }
            }
            Mode::Stand => {
                // Top up provisions while they're low and we can afford it.
                if game.state.provisions < 40.0 && game.state.cash > 12.0 {
                    Action::RestAndResupply(8.0)
                } else {
                    Action::LeaveStand
                }
            }
            Mode::Moneylender => Action::Depart,
            _ => Action::Noop,
        }
    }
}

/// A fuzzing driver: picks a valid-ish action at random for the current mode and
/// fuzzes minigame outcomes. Good for shaking out panics and soft-locks. Seeded,
/// so a failing run reproduces.
pub struct RandomFuzz {
    rng: GameRng,
}

impl RandomFuzz {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: GameRng::from_seed(seed),
        }
    }
}

impl Policy for RandomFuzz {
    fn persona(&self) -> Persona {
        Persona::Reckless
    }

    fn decide(&mut self, game: &Game, _market: Option<&Market>) -> Action {
        if let Some(a) = minigame_action(game, self.rng.ri(120) as u64) {
            return a;
        }
        let g = self.rng.ri(NUM_GOODS as i64) as usize;
        match game.mode {
            Mode::Pittsburgh => {
                if !game.has_boat() {
                    Action::Build {
                        kind: BoatKind::ALL[self.rng.ri(3) as usize],
                        crew: 1 + self.rng.ri(5),
                    }
                } else if self.rng.one_in(3.0) {
                    Action::Depart
                } else {
                    Action::Trade {
                        good: g,
                        side: Side::Buy,
                        qty: self.rng.ri(game.max_buy(g).max(1) + 1),
                    }
                }
            }
            Mode::Town | Mode::Trade { .. } => match self.rng.ri(4) {
                0 => Action::Trade {
                    good: g,
                    side: Side::Buy,
                    qty: self.rng.ri(game.max_buy(g).max(1) + 1),
                },
                1 => Action::Trade {
                    good: g,
                    side: Side::Sell,
                    qty: self.rng.ri(game.state.hold[g].max(1) + 1),
                },
                2 => Action::SetRiverConvoy(self.rng.one_in(2.0)),
                _ => Action::Depart,
            },
            Mode::Natchez => match self.rng.ri(4) {
                0 => Action::Trade {
                    good: g,
                    side: Side::Sell,
                    qty: self.rng.ri(game.state.hold[g].max(1) + 1),
                },
                1 if game.has_boat() => Action::SellBoat,
                2 if game.state.cash > 5.0 => Action::Gamble(5.0),
                _ => {
                    if game.has_boat() {
                        Action::SellBoat
                    } else {
                        Action::SetOutOnTrace
                    }
                }
            },
            Mode::Falls => match self.rng.ri(3) {
                0 => Action::FallsPilot(8.0),
                1 => Action::FallsRun,
                _ => Action::FallsWait,
            },
            Mode::GrandTower => {
                if self.rng.one_in(2.0) {
                    Action::GrandTowerTreat(2.0)
                } else {
                    Action::GrandTowerDuck
                }
            }
            Mode::CaveInRock => match self.rng.ri(3) {
                0 => Action::CaveTake,
                1 => Action::CaveHire(5.0),
                _ => Action::CaveRun,
            },
            Mode::TraceHub => match self.rng.ri(3) {
                0 => Action::SetPace(if self.rng.one_in(2.0) {
                    Pace::Steady
                } else {
                    Pace::Hard
                }),
                1 => Action::SetGrouped(self.rng.one_in(2.0)),
                _ => Action::TravelDay,
            },
            Mode::Stand => match self.rng.ri(3) {
                0 => Action::RestAndResupply(8.0),
                1 => Action::BuyHorse(14.0),
                _ => Action::LeaveStand,
            },
            Mode::Moneylender => Action::Depart,
            _ => Action::Noop,
        }
    }
}
