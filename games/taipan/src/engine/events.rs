//! Voyages (pirates → Li Yuen → storm → arrival) and the per-arrival
//! event chain, staged so interactive prompts can interrupt and resume.

use super::combat::{Battle, Fleet};
use super::interaction::Interaction;
use super::prices;
use super::state::{fmt_money, Mode, AT_SEA, HONG_KONG, ITEM_NAMES, PORT_NAMES};
use super::{scoring, Game, Voyage, VoyageStage};

impl Game {
    /// Weigh anchor. Fails (with the original's complaint) if overloaded.
    pub fn set_sail(&mut self, destination: usize) -> Result<(), String> {
        if !(1..=7).contains(&destination) || destination == self.state.port {
            return Err("You're already here, Taipan.".into());
        }
        if self.state.free_hold() < 0 {
            return Err("Your ship is overloaded, Taipan!!".into());
        }
        self.state.port = AT_SEA;
        self.voyage = Some(Voyage {
            destination,
            stage: VoyageStage::Pirates,
        });
        self.continue_voyage();
        Ok(())
    }

    /// Advance the at-sea event chain until something needs the player
    /// (combat, paced messages) or we make port.
    pub(crate) fn continue_voyage(&mut self) {
        loop {
            let Some(voyage) = self.voyage else { return };
            match voyage.stage {
                VoyageStage::Pirates => {
                    self.set_stage(VoyageStage::LiYuen);
                    // 1 in BP (10 cash start / 7 guns start)
                    if self.rng.one_in(self.state.pirate_divisor) {
                        self.battle = Some(Battle::new(Fleet::Generic, &self.state, &mut self.rng));
                        self.mode = Mode::Combat;
                        return;
                    }
                }
                VoyageStage::LiYuen | VoyageStage::LiYuenForced => {
                    let forced = voyage.stage == VoyageStage::LiYuenForced;
                    self.set_stage(VoyageStage::Storm);
                    // 1 in 4, or 1 in 12 under Li Yuen's protection
                    let odds = 4.0 + 8.0 * self.state.li_yuen as u8 as f64;
                    if forced || self.rng.one_in(odds) {
                        if self.state.li_yuen {
                            self.message("Li Yuen's pirates, Taipan!!");
                            self.message("Good joss!! They let us be!!");
                            self.mode = Mode::Interaction;
                            return;
                        }
                        self.battle = Some(Battle::new(Fleet::LiYuen, &self.state, &mut self.rng));
                        self.mode = Mode::Combat;
                        return;
                    }
                }
                VoyageStage::Storm => {
                    self.set_stage(VoyageStage::Arrive);
                    if self.rng.one_in(10.0) {
                        self.message("Storm, Taipan!!");
                        if self.rng.one_in(30.0) {
                            self.message("I think we're going down!!");
                            // Sinks when R(damage/capacity * 3) != 0
                            let ratio = self.state.damage / self.state.capacity as f64 * 3.0;
                            if self.rng.r(ratio) != 0 {
                                self.message("We're going down, Taipan!!");
                                self.outcome = Some(scoring::end_game(
                                    &self.state,
                                    false,
                                    "Your ship was lost in a storm at sea.",
                                ));
                                self.voyage = None;
                                self.mode = Mode::Interaction;
                                return;
                            }
                        }
                        self.message("We made it!!");
                        if self.rng.one_in(3.0) {
                            // Blown off course to a random other port.
                            let mut new_dest = self.rng.ri(7) as usize + 1;
                            while new_dest == voyage.destination {
                                new_dest = self.rng.ri(7) as usize + 1;
                            }
                            if let Some(v) = self.voyage.as_mut() {
                                v.destination = new_dest;
                            }
                            self.message(format!(
                                "We've been blown off course to {}!",
                                PORT_NAMES[new_dest]
                            ));
                        }
                        self.mode = Mode::Interaction;
                        return;
                    }
                }
                VoyageStage::Arrive => {
                    let destination = voyage.destination;
                    self.voyage = None;
                    self.state.port = destination;
                    self.advance_time();
                    self.message(format!("Arriving at {}...", PORT_NAMES[destination]));
                    self.arrival_stage = Some(0);
                    self.mode = Mode::Interaction;
                    return;
                }
            }
        }
    }

    fn set_stage(&mut self, stage: VoyageStage) {
        if let Some(v) = self.voyage.as_mut() {
            v.stage = stage;
        }
    }

    /// One month passes: bank interest 0.5%, Wu's debt grows 10%,
    /// and each new year the seas get meaner and prices drift up.
    fn advance_time(&mut self) {
        let s = &mut self.state;
        // INT(BA + BA*.005) / INT(DW + DW*.1); both stay whole numbers.
        s.bank += (s.bank * 0.005).floor();
        s.debt += (s.debt * 0.1).floor();
        s.months += 1;
        s.month += 1;
        if s.month > 12 {
            s.month = 1;
            s.year += 1;
            s.ec += 10.0;
            s.ed += 0.5;
            prices::inflate_base_ranks(s, &mut self.rng);
        }
    }

    /// Deal the next arrival event. Each stage queues at most one prompt;
    /// `after_queue` calls back here once the player has dealt with it,
    /// so later stages see the cash/state changes of earlier ones.
    pub(crate) fn continue_arrival(&mut self) {
        while let Some(stage) = self.arrival_stage {
            self.arrival_stage = Some(stage + 1);
            match stage {
                // Li Yuen asks for a donation to the sea goddess (Hong Kong).
                0 => {
                    let s = &self.state;
                    if s.port == HONG_KONG && !s.li_yuen && s.cash > 0.0 {
                        let ti = s.months as f64;
                        let amount = if s.months <= 12 {
                            self.rng.r(s.cash / 1.8) as f64
                        } else {
                            (self.rng.r(1000.0 * ti) as f64) + 1000.0 * ti
                        };
                        if amount > 0.0 {
                            self.pending
                                .push_back(Interaction::LiYuenDonation { amount });
                        }
                    }
                }
                // McHenry from the shipyards (Hong Kong, damaged ship).
                1 => {
                    let s = &self.state;
                    if s.port == HONG_KONG && s.damage > 0.0 {
                        let t = (s.months as f64 + 3.0) / 4.0;
                        let rate = ((self.rng.r(60.0 * t) as f64 + 25.0 * t)
                            * s.capacity as f64
                            / 50.0)
                            .floor()
                            .max(1.0);
                        self.pending.push_back(Interaction::McHenryRepair { rate });
                    }
                }
                // Wu's braves come calling when the debt tops 10,000.
                2 => {
                    if self.state.port == HONG_KONG
                        && self.state.debt >= 10_000.0
                        && !self.state.wu_warned
                    {
                        self.state.wu_warned = true;
                        let braves = self.rng.ri(100) + 50;
                        self.message(format!(
                            "Elder Brother Wu has sent {braves} braves to escort you to the Wu mansion, Taipan. Elder Brother Wu reminds you of the Confucian ideal of personal worthiness, and how this applies to paying one's debts."
                        ));
                    }
                }
                // "Do you have business with Elder Brother Wu?" (Hong Kong).
                3 => {
                    if self.state.port == HONG_KONG {
                        self.pending.push_back(Interaction::WuVisitPrompt);
                    }
                }
                // A bigger ship: +50 capacity, fresh hull.
                4 => {
                    let s = &self.state;
                    let damaged = s.damage > 0.0;
                    let mult = (s.capacity / 50) * damaged as i64 + 1;
                    let ti = s.months as f64;
                    let price =
                        (1000.0 + self.rng.r(1000.0 * (ti + 5.0) / 6.0) as f64) * mult as f64;
                    if s.cash >= price && self.rng.one_in(4.0) {
                        self.pending
                            .push_back(Interaction::ShipOffer { price, damaged });
                    }
                }
                // One more gun (10 hold units).
                5 => {
                    let ti = self.state.months as f64;
                    let price = (self.rng.r(1000.0 * (ti + 5.0) / 6.0) + 500) as f64;
                    if self.state.cash >= price && self.rng.one_in(3.0) {
                        self.pending.push_back(Interaction::GunOffer { price });
                    }
                }
                // The authorities seize opium aboard (anywhere but Hong Kong).
                6 => {
                    if self.state.port != HONG_KONG
                        && self.state.hold[0] > 0
                        && self.rng.one_in(18.0)
                    {
                        let fine = self.rng.r(self.state.cash / 1.8).max(0) as f64;
                        self.state.hold[0] = 0;
                        self.state.cash -= fine;
                        self.message(format!(
                            "Bad joss!! The local authorities have seized your Opium cargo and have also fined you {}, Taipan!",
                            fmt_money(fine)
                        ));
                    }
                }
                // Thieves hit the Hong Kong godown.
                7 => {
                    if self.state.warehouse_used() > 0 && self.rng.one_in(50.0) {
                        for stock in self.state.warehouse.iter_mut() {
                            let kept = self.rng.r(*stock as f64 / 1.8);
                            *stock = kept;
                        }
                        self.message(
                            "Messenger reports large theft from your warehouse, Taipan!!",
                        );
                    }
                }
                // Li Yuen's protection can lapse; his lieutenant nags from afar.
                8 => {
                    if self.state.li_yuen {
                        // LI = LI AND R(20): lapses 1 time in 20.
                        self.state.li_yuen = self.rng.ri(20) != 0;
                    } else if self.state.port != HONG_KONG && self.rng.one_in(4.0) {
                        self.message(
                            "Li Yuen has sent a Lieutenant, Taipan. He says his admiral wishes to see you in Hong Kong, posthaste!",
                        );
                    }
                }
                // Fresh prices for this port, with the occasional shock.
                9 => {
                    prices::generate_prices(&mut self.state, &mut self.rng);
                    if self.rng.one_in(9.0) {
                        let item = self.rng.ri(4) as usize;
                        if self.rng.ri(2) == 0 {
                            self.state.prices[item] = (self.state.prices[item] / 5.0).floor();
                            self.message(format!(
                                "Taipan!! The price of {} has dropped to {}!!",
                                ITEM_NAMES[item],
                                fmt_money(self.state.prices[item])
                            ));
                        } else {
                            self.state.prices[item] *= (self.rng.ri(5) + 5) as f64;
                            self.message(format!(
                                "Taipan!! The price of {} has risen to {}!!",
                                ITEM_NAMES[item],
                                fmt_money(self.state.prices[item])
                            ));
                        }
                    }
                }
                // Flashing that much cash around the docks is asking for it.
                10 => {
                    if self.state.cash > 25_000.0 && self.rng.one_in(20.0) {
                        let lost = self.rng.r(self.state.cash / 1.4) as f64;
                        self.state.cash -= lost;
                        self.message(format!(
                            "Bad joss!! You've been beaten up and robbed of {} in cash, Taipan!!",
                            fmt_money(lost)
                        ));
                    }
                }
                _ => {
                    self.arrival_stage = None;
                    self.mode = Mode::Port;
                    return;
                }
            }
            if !self.pending.is_empty() {
                self.mode = Mode::Interaction;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::interaction::{Interaction, Response};
    use crate::engine::state::StartChoice;

    fn started(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.start("Test & Co.".into(), StartChoice::Cash);
        g
    }

    /// Answer prompts (declining offers) until the game settles in port.
    fn settle(g: &mut Game) {
        let mut guard = 0;
        while g.mode == Mode::Interaction {
            let response = match g.pending.front() {
                Some(Interaction::Message(_)) => Response::Acknowledge,
                Some(Interaction::McHenryRepair { .. }) => Response::Amount(0.0),
                _ => Response::No,
            };
            g.resolve(response);
            guard += 1;
            assert!(guard < 100, "interaction queue never drained");
        }
    }

    #[test]
    fn new_game_reaches_port_with_prices() {
        let mut g = started(1);
        settle(&mut g);
        assert_eq!(g.mode, Mode::Port);
        assert!(g.state.prices.iter().all(|&p| p > 0.0));
        assert_eq!(g.state.port, HONG_KONG);
        assert_eq!(g.state.months, 1, "starting arrival must not advance time");
    }

    #[test]
    fn buying_and_selling_clamp_and_balance() {
        let mut g = started(2);
        settle(&mut g);
        let item = 3; // general cargo, cheapest
        let price = g.state.prices[item];
        let max = g.max_buy(item);
        assert_eq!(max, (g.state.cash / price).floor() as i64);
        g.buy(item, max + 1000); // over-ask clamps
        assert_eq!(g.state.hold[item], max);
        assert!(g.state.cash < price, "spent down below one more unit");
        g.sell(item, i64::MAX);
        assert_eq!(g.state.hold[item], 0);
    }

    #[test]
    fn sailing_advances_time_and_interest() {
        let mut g = started(3);
        settle(&mut g);
        g.state.bank = 1000.0;
        let debt_before = g.state.debt;
        let months_before = g.state.months;
        g.set_sail(2).unwrap();
        // Drive through whatever the sea throws at us.
        let mut guard = 0;
        while g.mode != Mode::Port && g.outcome.is_none() {
            match g.mode {
                Mode::Combat => {
                    let _ = g.battle_order(crate::engine::interaction::Order::Run);
                    if g.battle.as_ref().and_then(|b| b.outcome).is_some() {
                        g.finish_battle();
                    }
                }
                Mode::Interaction => settle(&mut g),
                Mode::Wu => g.leave_wu(),
                _ => break,
            }
            guard += 1;
            assert!(guard < 500, "voyage never completed (mode {:?})", g.mode);
        }
        if g.outcome.is_none() {
            assert_eq!(g.state.months, months_before + 1);
            assert_eq!(g.state.bank, 1005.0, "0.5% bank interest");
            assert_eq!(g.state.debt, (debt_before * 1.1).floor(), "10% debt growth");
            assert_eq!(g.state.port, 2);
        }
    }

    #[test]
    fn sail_rejects_overload_and_same_port() {
        let mut g = started(4);
        settle(&mut g);
        assert!(g.set_sail(HONG_KONG).is_err());
        g.state.hold[3] = 1000; // grossly overloaded
        assert!(g.set_sail(2).is_err());
    }

    #[test]
    fn wu_borrow_limit_and_repay() {
        let mut g = started(5);
        settle(&mut g);
        let cash = g.state.cash;
        let debt = g.state.debt;
        g.borrow(1e9); // clamps to 2x cash
        assert_eq!(g.state.cash, cash * 3.0);
        assert_eq!(g.state.debt, debt + cash * 2.0);
        g.repay(1e9); // clamps to min(cash, debt)
        assert!(g.state.debt >= 0.0 && g.state.cash >= 0.0);
    }

    #[test]
    fn wu_bailout_when_destitute() {
        let mut g = started(6);
        settle(&mut g);
        g.state.cash = 0.0;
        g.state.bank = 0.0;
        g.state.hold = [0; 4];
        g.state.warehouse = [0; 4];
        g.state.guns = 0;
        g.visit_wu();
        assert!(matches!(
            g.pending.front(),
            Some(Interaction::WuBailout { .. })
        ));
        let (offer, repay) = match g.pending.front().unwrap() {
            Interaction::WuBailout { offer, repay } => (*offer, *repay),
            _ => unreachable!(),
        };
        assert!((500.0..2000.0).contains(&offer));
        assert!(repay >= 1500.0);
        let debt_before = g.state.debt;
        g.resolve(Response::Yes);
        assert_eq!(g.state.cash, offer);
        assert_eq!(g.state.debt, debt_before + repay);
        assert_eq!(g.state.bailouts, 1);
    }

    #[test]
    fn refusing_bailout_ends_the_game() {
        let mut g = started(7);
        settle(&mut g);
        g.state.cash = 0.0;
        g.state.bank = 0.0;
        g.state.hold = [0; 4];
        g.state.warehouse = [0; 4];
        g.state.guns = 0;
        g.visit_wu();
        g.resolve(Response::No);
        assert!(g.outcome.is_some());
        assert_eq!(g.mode, Mode::GameOver);
    }

    #[test]
    fn li_yuen_donation_grants_protection() {
        // Find a seed whose first arrival includes the donation ask.
        for seed in 0..50 {
            let mut g = Game::new(seed);
            g.start("T".into(), StartChoice::Cash);
            if let Some(Interaction::LiYuenDonation { amount }) = g.pending.front().cloned() {
                assert!(amount <= g.state.cash, "first-year ask is R(cash/1.8)");
                let cash = g.state.cash;
                g.resolve(Response::Yes);
                assert!(g.state.li_yuen);
                assert_eq!(g.state.cash, cash - amount);
                return;
            }
        }
        panic!("no seed produced a Li Yuen donation on the first arrival");
    }

    #[test]
    fn warehouse_transfer_respects_capacity() {
        let mut g = started(8);
        settle(&mut g);
        g.state.hold[1] = 40;
        g.store_in_warehouse(1, 100);
        assert_eq!(g.state.warehouse[1], 40);
        assert_eq!(g.state.hold[1], 0);
        g.load_aboard(1, 15);
        assert_eq!(g.state.warehouse[1], 25);
        assert_eq!(g.state.hold[1], 15);
    }

    #[test]
    fn retirement_requires_a_million_at_hong_kong() {
        let mut g = started(9);
        settle(&mut g);
        assert!(!g.can_retire());
        g.state.cash = 2_000_000.0;
        assert!(g.can_retire());
        g.retire();
        let end = g.outcome.unwrap();
        assert!(end.retired);
        assert!(end.score > 0);
    }

    /// A scripted multi-voyage game stays internally consistent.
    #[test]
    fn scripted_game_smoke_test() {
        for seed in [11u64, 22, 33, 44, 55] {
            let mut g = started(seed);
            settle(&mut g);
            let mut guard = 0;
            'game: for dest in [2usize, 5, 1, 7, 1, 3, 1] {
                if g.state.port != dest && g.set_sail(dest).is_err() {
                    g.state.hold = [0; 4]; // jettison and retry
                    g.set_sail(dest).unwrap();
                }
                while g.mode != Mode::Port {
                    if g.outcome.is_some() {
                        break 'game;
                    }
                    match g.mode {
                        Mode::Combat => {
                            let _ = g.battle_order(crate::engine::interaction::Order::Run);
                            if g.battle.as_ref().and_then(|b| b.outcome).is_some() {
                                g.finish_battle();
                            }
                        }
                        Mode::Interaction => {
                            let response = match g.pending.front() {
                                Some(Interaction::Message(_)) => Response::Acknowledge,
                                Some(Interaction::McHenryRepair { .. }) => Response::Amount(0.0),
                                _ => Response::No,
                            };
                            g.resolve(response);
                        }
                        Mode::Wu => g.leave_wu(),
                        _ => break,
                    }
                    guard += 1;
                    assert!(guard < 5000, "seed {seed}: game wedged in {:?}", g.mode);
                }
                if g.outcome.is_some() {
                    break;
                }
                // Trade a little at every port.
                let item = 3;
                g.buy(item, g.max_buy(item).min(g.state.free_hold().max(0)));
                g.sell(item, g.state.hold[item] / 2);
                assert!(g.state.cash >= 0.0, "seed {seed}: cash went negative");
                assert!(g.state.free_hold() >= -0, "seed {seed}: overload outside buy");
            }
            // Game either ended or is sitting at a port — both are consistent.
            assert!(g.outcome.is_some() || g.mode == Mode::Port);
        }
    }
}
