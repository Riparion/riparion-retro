//! Sea battle state machine — pure, synchronous round resolution.
//! The UI paces the returned messages; all rules live here.

use serde::{Deserialize, Serialize};

use super::interaction::Order;
use super::rng::GameRng;
use super::state::{GameState, ITEM_NAMES};

pub const MAX_ON_SCREEN: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fleet {
    /// Generic hostiles (`F1 = 1`).
    Generic,
    /// Li Yuen's pirates (`F1 = 2`): tougher, hit harder, harder to shake.
    LiYuen,
}

impl Fleet {
    pub fn factor(self) -> f64 {
        match self {
            Fleet::Generic => 1.0,
            Fleet::LiYuen => 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnemyShip {
    pub health: f64,
    pub damage: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleOutcome {
    /// All enemies sunk; booty collected.
    Victory,
    /// Got away.
    Escaped,
    /// Li Yuen's fleet drove the attackers off (generic fleets only).
    LiYuenRescue,
    /// Ship lost; game over.
    Lost,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Battle {
    pub fleet: Fleet,
    /// Ships still hostile (`SN`).
    pub ships: i64,
    /// Original attacker count (`S0`).
    pub original: i64,
    /// Up to ten ships currently engaged on screen.
    pub on_screen: Vec<EnemyShip>,
    /// Pre-rolled prize money, paid only on victory (`BT`).
    pub booty: f64,
    /// Escape parameter (`OK`) and its per-attempt ramp (`IK`).
    pub ok: f64,
    pub ik: f64,
    /// Whether the previous order was Run/Throw (consecutive attempts improve odds).
    pub ran_last: bool,
    pub outcome: Option<BattleOutcome>,
}

impl Battle {
    /// `SN = R(cap/10 + guns) + 1` for generic fleets,
    /// `SN = R(cap/5 + guns) + 5` for Li Yuen's.
    pub fn new(fleet: Fleet, state: &GameState, rng: &mut GameRng) -> Self {
        let cap = state.capacity as f64;
        let guns = state.guns as f64;
        let ships = match fleet {
            Fleet::Generic => rng.r(cap / 10.0 + guns) + 1,
            Fleet::LiYuen => rng.r(cap / 5.0 + guns) + 5,
        };
        // BT = R(TI/4 * 1000 * SN^1.05) + R(1000) + 250
        let booty = rng.r(state.months as f64 / 4.0 * 1000.0 * (ships as f64).powf(1.05)) as f64
            + rng.r(1000.0) as f64
            + 250.0;
        let mut battle = Self {
            fleet,
            ships,
            original: ships,
            on_screen: Vec::new(),
            booty,
            ok: 0.0,
            ik: 0.0,
            ran_last: false,
            outcome: None,
        };
        battle.refill_screen(state, rng);
        battle
    }

    /// Each engaged ship's strength: `R(EC) + 20`.
    fn refill_screen(&mut self, state: &GameState, rng: &mut GameRng) {
        while (self.on_screen.len() as i64) < self.ships.min(MAX_ON_SCREEN as i64) {
            self.on_screen.push(EnemyShip {
                health: (rng.r(state.ec) + 20) as f64,
                damage: 0.0,
            });
        }
    }

    /// Remove `n` hostiles (fled/shaken): prefer off-screen reserves.
    fn remove_ships(&mut self, n: i64) {
        self.ships -= n;
        while self.on_screen.len() as i64 > self.ships {
            self.on_screen.pop();
        }
    }

    pub fn intro(&self) -> String {
        match self.fleet {
            Fleet::Generic => format!("{} hostile ships approaching, Taipan!!", self.ships),
            Fleet::LiYuen => format!("Li Yuen's pirates, Taipan!! {} ships of his fleet!!", self.ships),
        }
    }
}

/// Resolve one player order plus the enemy reply. Mutates `state` and `battle`,
/// stores any final outcome in `battle.outcome`, and returns paced messages.
pub fn resolve_order(
    state: &mut GameState,
    battle: &mut Battle,
    rng: &mut GameRng,
    order: Order,
) -> Vec<String> {
    let mut msgs = Vec::new();
    let f1 = battle.fleet.factor();

    match order {
        Order::Fight => {
            if state.guns == 0 {
                msgs.push("We have no guns, Taipan!!".into());
            } else {
                msgs.push("Aye, we'll fight 'em, Taipan.".into());
                let mut sunk = 0i64;
                for _ in 0..state.guns {
                    if battle.on_screen.is_empty() {
                        break;
                    }
                    let idx = rng.ri(battle.on_screen.len() as i64) as usize;
                    let ship = &mut battle.on_screen[idx];
                    ship.damage += (rng.ri(30) + 10) as f64;
                    if ship.damage >= ship.health {
                        battle.on_screen.remove(idx);
                        battle.ships -= 1;
                        sunk += 1;
                    }
                }
                battle.refill_screen(state, rng);
                if sunk > 0 {
                    msgs.push(format!("Sunk {sunk} of the buggers, Taipan!!"));
                } else {
                    msgs.push("Hit 'em, but didn't sink 'em, Taipan!!".into());
                }
                // Some may flee: only after first blood, never below a skirmish.
                if battle.ships > 0
                    && battle.ships != battle.original
                    && battle.ships >= 3
                    && rng.ri(battle.original) as f64 >= battle.ships as f64 * 0.6 / f1
                {
                    let fled = rng.r(battle.ships as f64 / 3.0 / f1) + 1;
                    battle.remove_ships(fled);
                    msgs.push(format!("{fled} ran away, Taipan!!"));
                }
            }
            battle.ran_last = false;
        }
        Order::Run | Order::Throw { .. } => {
            let mut bonus = 0.0;
            if let Order::Throw { item, amount } = order {
                let amount = amount.clamp(0, state.hold[item]);
                if amount > 0 {
                    state.hold[item] -= amount;
                    bonus = amount as f64 / 10.0;
                    msgs.push(format!(
                        "Threw {} of {} overboard, Taipan!",
                        amount, ITEM_NAMES[item]
                    ));
                } else {
                    msgs.push("There's nothing to throw, Taipan!!".into());
                }
            } else {
                msgs.push("Aye, we'll run, Taipan.".into());
            }
            if battle.ran_last {
                battle.ok += battle.ik;
                battle.ik += 1.0;
            } else {
                battle.ok = 3.0;
                battle.ik = 1.0;
            }
            battle.ok += bonus;
            battle.ran_last = true;
            if rng.r(battle.ok) > rng.ri(battle.ships) {
                msgs.push("We got away from 'em, Taipan!!".into());
                battle.outcome = Some(BattleOutcome::Escaped);
                return msgs;
            }
            msgs.push("Couldn't lose 'em!!".into());
            if battle.ships > 2 && rng.one_in(5.0) {
                let lost = rng.r(battle.ships as f64 / 2.0) + 1;
                battle.remove_ships(lost);
                msgs.push(format!("But we escaped from {lost} of 'em!"));
            }
        }
    }

    // Enemy reply.
    if battle.ships == 0 {
        msgs.push("We got 'em all, Taipan!!".into());
        msgs.push(format!(
            "We've captured some booty. It's worth {}!",
            super::state::fmt_money(battle.booty)
        ));
        state.cash += battle.booty;
        battle.outcome = Some(BattleOutcome::Victory);
        return msgs;
    }

    msgs.push("They're firing on us, Taipan!!".into());
    let mut firing = battle.ships.min(15) as f64;
    let damage_ratio = state.damage / state.capacity as f64;
    if state.guns > 0 && ((rng.ri(100) as f64) < damage_ratio * 100.0 || damage_ratio > 0.8) {
        state.guns -= 1;
        firing = 1.0;
        msgs.push("The buggers hit a gun, Taipan!!".into());
    }
    state.damage += rng.r(state.ed * firing * f1) as f64 + firing / 2.0;

    if state.seaworthiness() == 0 {
        msgs.push("The buggers got us, Taipan!!! It's all over now!!!".into());
        battle.outcome = Some(BattleOutcome::Lost);
        return msgs;
    }
    msgs.push("We've been hit, Taipan!!".into());

    if battle.fleet == Fleet::Generic && rng.one_in(20.0) {
        msgs.push("Li Yuen's fleet drove them off!".into());
        battle.outcome = Some(BattleOutcome::LiYuenRescue);
    }

    msgs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::StartChoice;

    fn setup(choice: StartChoice, seed: u64) -> (GameState, GameRng) {
        (GameState::new("T".into(), choice), GameRng::from_seed(seed))
    }

    #[test]
    fn fleet_sizes_match_formulas() {
        let (state, mut rng) = setup(StartChoice::Guns, 5);
        for _ in 0..200 {
            let b = Battle::new(Fleet::Generic, &state, &mut rng);
            // R(60/10 + 5) + 1 = 1..=11
            assert!((1..=11).contains(&b.ships), "generic {}", b.ships);
            let b = Battle::new(Fleet::LiYuen, &state, &mut rng);
            // R(60/5 + 5) + 5 = 5..=21
            assert!((5..=21).contains(&b.ships), "li yuen {}", b.ships);
            assert!(b.booty >= 250.0);
            assert_eq!(b.on_screen.len() as i64, b.ships.min(10));
        }
    }

    #[test]
    fn fighting_without_guns_is_futile() {
        let (mut state, mut rng) = setup(StartChoice::Cash, 1);
        let mut battle = Battle::new(Fleet::Generic, &state, &mut rng);
        let before = battle.ships;
        let msgs = resolve_order(&mut state, &mut battle, &mut rng, Order::Fight);
        assert!(msgs.iter().any(|m| m.contains("no guns")));
        assert_eq!(battle.ships, before);
        assert!(state.damage > 0.0, "enemies still fire back");
    }

    #[test]
    fn victory_pays_booty() {
        let (mut state, mut rng) = setup(StartChoice::Guns, 3);
        state.guns = 50; // overwhelming firepower
        let mut battle = Battle::new(Fleet::Generic, &state, &mut rng);
        let booty = battle.booty;
        let cash_before = state.cash;
        for _ in 0..50 {
            resolve_order(&mut state, &mut battle, &mut rng, Order::Fight);
            if battle.outcome.is_some() {
                break;
            }
        }
        assert_eq!(battle.outcome, Some(BattleOutcome::Victory));
        assert_eq!(state.cash, cash_before + booty);
        assert_eq!(battle.ships, 0);
    }

    #[test]
    fn running_eventually_escapes_or_ends() {
        let (mut state, mut rng) = setup(StartChoice::Cash, 11);
        let mut battle = Battle::new(Fleet::Generic, &state, &mut rng);
        for _ in 0..100 {
            resolve_order(&mut state, &mut battle, &mut rng, Order::Run);
            if battle.outcome.is_some() {
                break;
            }
        }
        assert!(battle.outcome.is_some(), "battle should resolve");
    }

    #[test]
    fn throwing_cargo_jettisons_and_clamps() {
        let (mut state, mut rng) = setup(StartChoice::Cash, 7);
        state.hold[1] = 30; // silk
        let mut battle = Battle::new(Fleet::Generic, &state, &mut rng);
        resolve_order(
            &mut state,
            &mut battle,
            &mut rng,
            Order::Throw { item: 1, amount: 999 },
        );
        assert_eq!(state.hold[1], 0, "throw clamps to what's aboard");
    }

    #[test]
    fn heavy_damage_loses_the_ship() {
        let (mut state, mut rng) = setup(StartChoice::Cash, 2);
        state.damage = 59.5; // capacity 60 — one hit from doom
        let mut battle = Battle::new(Fleet::LiYuen, &state, &mut rng);
        let msgs = resolve_order(&mut state, &mut battle, &mut rng, Order::Fight);
        assert_eq!(battle.outcome, Some(BattleOutcome::Lost));
        assert!(msgs.iter().any(|m| m.contains("got us")));
    }
}
