//! A single, flat vocabulary of every decision a player can make, and a
//! [`Game::apply`] dispatcher that routes each to the engine method the UI
//! already calls. This is what lets an autonomous [`Policy`](crate::policy) drive
//! the very same engine a human drives: the UI maps button presses to these, and
//! a bot returns them from `decide`.
//!
//! The set is derived directly from the scripted golden-trace driver in
//! `tests.rs` (every hub branch and every minigame/interaction resolution), so it
//! is exhaustive over the real decision points. `apply` adds no game logic — it
//! is a thin match onto existing public methods.

use crate::interaction::Response;
use crate::market_link::Side;
use crate::state::{BoatKind, Pace};
use crate::Game;

/// One player decision. See [`Game::apply`].
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // ----- Pittsburgh: build & outfit -----
    Build { kind: BoatKind, crew: i64 },

    // ----- Trading (any market) -----
    Trade { good: usize, side: Side, qty: i64 },

    // ----- Moneylender -----
    Borrow(f64),
    Repay(f64),

    // ----- River movement -----
    SetRiverConvoy(bool),
    Depart,

    // ----- Boatyard repair -----
    RepairInPort,
    SelfRepair,

    // ----- Falls of the Ohio -----
    FallsPilot(f64),
    FallsRun,
    FallsWait,

    // ----- Grand Tower initiation -----
    GrandTowerTreat(f64),
    GrandTowerDuck,

    // ----- Cave-in-Rock relay-pilot con -----
    CaveTake,
    CaveHire(f64),
    CaveRun,

    // ----- Natchez -----
    SellBoat,
    BuyHorse(f64),
    Gamble(f64),
    SetOutOnTrace,

    // ----- Natchez Trace -----
    SetPace(Pace),
    SetGrouped(bool),
    TravelDay,
    RestAndResupply(f64),
    LeaveStand,

    // ----- Shared: interaction prompts -----
    Respond(Response),

    // ----- Shared: minigame resolutions -----
    ResolveSteady { steady: bool, accuracy: f64 },
    ResolveQuick { reaction_secs: f64, hit: bool },
    ResolveCrowd { cleared: bool, accuracy: f64 },
    ResolveTiming { hit: bool, accuracy: f64 },
    ResolveSequence { correct_prefix: usize, length: usize, perfect: bool },
    ResolveBrigade { contained: bool, leaked: usize, capacity: usize },
    ResolveHeave { opened: bool, slips: usize, grip_left: f64 },
    ResolveHotCold { found: bool, probes_used: usize, budget: usize },
    ResolveHunter { hit: bool, shots_fired: usize },

    /// Wait — make no change this step. A safety valve for a policy with no valid
    /// move; the world's tick/guard bounds keep it from spinning forever.
    Noop,
}

impl Game {
    /// Apply one [`Action`] by dispatching to the existing engine method. No new
    /// game logic lives here — buy/sell/resolve/etc. are unchanged.
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Build { kind, crew } => {
                let _ = self.build(kind, crew);
            }
            Action::Trade { good, side, qty } => match side {
                Side::Buy => self.buy(good, qty),
                Side::Sell => self.sell(good, qty),
            },
            Action::Borrow(a) => self.borrow(a),
            Action::Repay(a) => self.repay(a),
            Action::SetRiverConvoy(c) => self.set_river_convoy(c),
            Action::Depart => self.depart(),
            Action::RepairInPort => self.repair_in_port(),
            Action::SelfRepair => self.self_repair(),
            Action::FallsPilot(fee) => self.falls_pilot(fee),
            Action::FallsRun => self.falls_run(),
            Action::FallsWait => self.falls_wait(),
            Action::GrandTowerTreat(fee) => self.grand_tower_treat(fee),
            Action::GrandTowerDuck => self.grand_tower_duck(),
            Action::CaveTake => self.cave_take(),
            Action::CaveHire(fee) => self.cave_hire(fee),
            Action::CaveRun => self.cave_run(),
            Action::SellBoat => self.sell_boat(),
            Action::BuyHorse(p) => self.buy_horse(p),
            Action::Gamble(a) => self.gamble(a),
            Action::SetOutOnTrace => self.set_out_on_trace(),
            Action::SetPace(p) => self.set_pace(p),
            Action::SetGrouped(g) => self.set_grouped(g),
            Action::TravelDay => self.travel_day(),
            Action::RestAndResupply(c) => self.rest_and_resupply(c),
            Action::LeaveStand => self.leave_stand(),
            Action::Respond(r) => self.resolve(r),
            Action::ResolveSteady { steady, accuracy } => self.resolve_steady(steady, accuracy),
            Action::ResolveQuick { reaction_secs, hit } => self.resolve_quick(reaction_secs, hit),
            Action::ResolveCrowd { cleared, accuracy } => self.resolve_crowd(cleared, accuracy),
            Action::ResolveTiming { hit, accuracy } => self.resolve_timing(hit, accuracy),
            Action::ResolveSequence {
                correct_prefix,
                length,
                perfect,
            } => self.resolve_sequence(correct_prefix, length, perfect),
            Action::ResolveBrigade {
                contained,
                leaked,
                capacity,
            } => self.resolve_brigade(contained, leaked, capacity),
            Action::ResolveHeave {
                opened,
                slips,
                grip_left,
            } => self.resolve_heave(opened, slips, grip_left),
            Action::ResolveHotCold {
                found,
                probes_used,
                budget,
            } => self.resolve_hotcold(found, probes_used, budget),
            Action::ResolveHunter { hit, shots_fired } => self.resolve_hunter(hit, shots_fired),
            Action::Noop => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Carryover;

    fn started(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.begin_with("Tester".into(), Carryover::fresh());
        g
    }

    #[test]
    fn apply_matches_calling_the_method_directly() {
        // Build + buy via Action should equal build + buy via the methods.
        let mut by_action = started(101);
        by_action.apply(Action::Build {
            kind: BoatKind::Flatboat,
            crew: 3,
        });
        by_action.apply(Action::Trade {
            good: 0,
            side: Side::Buy,
            qty: 5,
        });

        let mut by_method = started(101);
        let _ = by_method.build(BoatKind::Flatboat, 3);
        by_method.buy(0, 5);

        assert_eq!(by_action.state.hold, by_method.state.hold);
        assert_eq!(by_action.state.cash, by_method.state.cash);
        assert_eq!(by_action.mode, by_method.mode);
    }

    #[test]
    fn noop_is_a_no_op() {
        let mut g = started(5);
        let before = g.clone();
        g.apply(Action::Noop);
        assert_eq!(g, before);
    }
}
