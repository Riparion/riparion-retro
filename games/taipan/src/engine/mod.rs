//! Pure game engine — no UI or platform dependencies. The UI layer owns a
//! `Game` in a signal and calls these methods; everything here is host-testable.

pub mod combat;
pub mod events;
pub mod interaction;
pub mod prices;
pub use retro_kit::rng;
pub mod scoring;
pub mod state;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use combat::{Battle, BattleOutcome};
use interaction::{Interaction, Order, Response};
use rng::GameRng;
use state::{EndGame, GameState, Mode, StartChoice, GUN_HOLD_UNITS, HONG_KONG, WAREHOUSE_CAPACITY};

/// Where we are in a voyage's event chain (so combat can interrupt and resume).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoyageStage {
    Pirates,
    LiYuen,
    /// Li Yuen's fleet shows up unconditionally (after his fleet drives off generic pirates).
    LiYuenForced,
    Storm,
    Arrive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voyage {
    pub destination: usize,
    pub stage: VoyageStage,
}

/// The complete game: world state, UI mode, pending prompts, and RNG.
/// Fully serializable so a refresh resumes exactly in place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub state: GameState,
    pub mode: Mode,
    pub pending: VecDeque<Interaction>,
    pub battle: Option<Battle>,
    pub voyage: Option<Voyage>,
    /// Arrival event chain position; `Some(n)` while events are still being dealt.
    pub arrival_stage: Option<u8>,
    pub outcome: Option<EndGame>,
    pub rng: GameRng,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            state: GameState::new(String::new(), StartChoice::Cash),
            mode: Mode::Splash,
            pending: VecDeque::new(),
            battle: None,
            voyage: None,
            arrival_stage: None,
            outcome: None,
            rng: GameRng::from_seed(seed),
        }
    }

    /// Begin a fresh game. Runs the first Hong Kong arrival chain
    /// (Li Yuen, Wu, offers...) without advancing time.
    pub fn start(&mut self, firm: String, choice: StartChoice) {
        self.state = GameState::new(firm, choice);
        self.pending.clear();
        self.battle = None;
        self.voyage = None;
        self.outcome = None;
        self.arrival_stage = Some(0);
        self.continue_arrival();
    }

    // ----- Port actions (engine re-clamps everything; UI input is advisory) -----

    /// Max units of `item` affordable right now.
    pub fn max_buy(&self, item: usize) -> i64 {
        let price = self.state.prices[item];
        if price <= 0.0 {
            return 0;
        }
        (self.state.cash / price).floor() as i64
    }

    pub fn buy(&mut self, item: usize, qty: i64) {
        let qty = qty.clamp(0, self.max_buy(item));
        self.state.cash -= qty as f64 * self.state.prices[item];
        self.state.hold[item] += qty;
    }

    pub fn sell(&mut self, item: usize, qty: i64) {
        let qty = qty.clamp(0, self.state.hold[item]);
        self.state.cash += qty as f64 * self.state.prices[item];
        self.state.hold[item] -= qty;
    }

    pub fn store_in_warehouse(&mut self, item: usize, qty: i64) {
        let room = WAREHOUSE_CAPACITY - self.state.warehouse_used();
        let qty = qty.clamp(0, self.state.hold[item].min(room));
        self.state.hold[item] -= qty;
        self.state.warehouse[item] += qty;
    }

    pub fn load_aboard(&mut self, item: usize, qty: i64) {
        let qty = qty.clamp(0, self.state.warehouse[item]);
        self.state.warehouse[item] -= qty;
        self.state.hold[item] += qty;
    }

    pub fn deposit(&mut self, amount: f64) {
        let amount = amount.clamp(0.0, self.state.cash);
        self.state.cash -= amount;
        self.state.bank += amount;
    }

    pub fn withdraw(&mut self, amount: f64) {
        let amount = amount.clamp(0.0, self.state.bank);
        self.state.bank -= amount;
        self.state.cash += amount;
    }

    /// Wu lends at most twice your cash in hand.
    pub fn max_borrow(&self) -> f64 {
        self.state.cash * 2.0
    }

    pub fn borrow(&mut self, amount: f64) {
        let amount = amount.clamp(0.0, self.max_borrow());
        self.state.cash += amount;
        self.state.debt += amount;
    }

    pub fn repay(&mut self, amount: f64) {
        let amount = amount.clamp(0.0, self.state.cash.min(self.state.debt));
        self.state.cash -= amount;
        self.state.debt -= amount;
    }

    /// Open the Wu screen (Hong Kong). If utterly broke, Wu intervenes instead.
    pub fn visit_wu(&mut self) {
        let s = &self.state;
        let broke = s.cash == 0.0
            && s.bank == 0.0
            && s.cargo_aboard() == 0
            && s.warehouse_used() == 0
            && s.guns == 0;
        if broke {
            let offer = (self.rng.ri(1500) + 500) as f64;
            let repay = (self.rng.ri(2000) * (self.state.bailouts as i64 + 1) + 1500) as f64;
            self.pending.push_front(Interaction::WuBailout { offer, repay });
            self.mode = Mode::Interaction;
        } else {
            self.mode = Mode::Wu;
        }
    }

    /// Leave the Wu screen. Carrying a huge debt around Wu's braves is unwise.
    pub fn leave_wu(&mut self) {
        if self.state.debt > 20_000.0 && self.rng.one_in(5.0) {
            let killed = self.rng.ri(3) + 1;
            self.state.cash = 0.0;
            self.pending.push_back(Interaction::Message(format!(
                "Bad joss!! {killed} of your bodyguards have been killed by cutthroats and you have been robbed of all of your cash, Taipan!!"
            )));
        }
        self.after_queue();
    }

    pub fn can_retire(&self) -> bool {
        self.state.port == HONG_KONG && self.state.net_worth() >= 1_000_000.0
    }

    pub fn retire(&mut self) {
        if !self.can_retire() {
            return;
        }
        self.outcome = Some(scoring::end_game(
            &self.state,
            true,
            "You're a MILLIONAIRE! You have retired in glory.",
        ));
        self.mode = Mode::GameOver;
    }

    // ----- Interactions -----

    /// Apply the player's answer to the queue head, then move the game along.
    pub fn resolve(&mut self, response: Response) {
        let Some(interaction) = self.pending.pop_front() else {
            self.after_queue();
            return;
        };
        match interaction {
            Interaction::Message(_) => {}
            Interaction::LiYuenDonation { amount } => {
                if response == Response::Yes {
                    if amount <= self.state.cash {
                        self.state.cash -= amount;
                        self.state.li_yuen = true;
                    } else {
                        self.pending.push_front(Interaction::WuCoverDonation {
                            amount,
                            shortfall: amount - self.state.cash,
                        });
                    }
                }
            }
            Interaction::WuCoverDonation { shortfall, .. } => {
                if response == Response::Yes {
                    self.state.debt += shortfall;
                    self.state.cash = 0.0;
                    self.state.li_yuen = true;
                    self.message("Elder Brother Wu has given Li Yuen the difference between what he wanted and all of your cash, and added the same amount to your debt.");
                } else {
                    self.message("Very well. Elder Brother Wu will not pay Li Yuen the difference. I would be very careful if I were you, Taipan.");
                }
            }
            Interaction::McHenryRepair { rate } => {
                if let Response::Amount(w) = response {
                    let max = rate * self.state.damage + 1.0;
                    let w = w.clamp(0.0, self.state.cash.min(max));
                    if w > 0.0 {
                        let repaired = (w / rate + 0.5).floor();
                        self.state.damage = (self.state.damage - repaired).max(0.0);
                        self.state.cash -= w;
                    }
                }
            }
            Interaction::WuVisitPrompt => {
                if response == Response::Yes {
                    self.visit_wu();
                    return; // queue resumes when the player leaves Wu
                }
            }
            Interaction::WuBailout { offer, repay } => {
                if response == Response::Yes {
                    self.state.cash += offer;
                    self.state.debt += repay;
                    self.state.bailouts += 1;
                    self.message("Very well, Taipan. Good joss!!");
                } else {
                    self.outcome = Some(scoring::end_game(
                        &self.state,
                        false,
                        "You refused Elder Brother Wu's terms. Very well, Taipan, the game is over!",
                    ));
                }
            }
            Interaction::ShipOffer { price, .. } => {
                if response == Response::Yes && self.state.cash >= price {
                    self.state.cash -= price;
                    self.state.capacity += 50;
                    self.state.damage = 0.0;
                }
            }
            Interaction::GunOffer { price } => {
                if response == Response::Yes && self.state.cash >= price {
                    if self.state.free_hold() >= GUN_HOLD_UNITS {
                        self.state.cash -= price;
                        self.state.guns += 1;
                    } else {
                        self.message("Your ship would be overburdened, Taipan!!");
                    }
                }
            }
        }
        if self.pending.is_empty() {
            self.after_queue();
        } else {
            self.mode = Mode::Interaction;
        }
    }

    pub(crate) fn message(&mut self, text: impl Into<String>) {
        self.pending.push_back(Interaction::Message(text.into()));
    }

    /// The queue just drained: resume whatever was interrupted.
    pub(crate) fn after_queue(&mut self) {
        if self.outcome.is_some() {
            self.mode = Mode::GameOver;
        } else if self.voyage.is_some() {
            self.continue_voyage();
        } else if self.arrival_stage.is_some() {
            self.continue_arrival();
        } else if !self.pending.is_empty() {
            self.mode = Mode::Interaction;
        } else {
            self.mode = Mode::Port;
        }
    }

    // ----- Combat -----

    /// Resolve one combat round; returns paced messages for the UI.
    pub fn battle_order(&mut self, order: Order) -> Vec<String> {
        let Some(battle) = self.battle.as_mut() else {
            return Vec::new();
        };
        combat::resolve_order(&mut self.state, battle, &mut self.rng, order)
    }

    /// Called by the UI after pacing the final round's messages.
    pub fn finish_battle(&mut self) {
        let Some(battle) = self.battle.take() else {
            return;
        };
        match battle.outcome {
            Some(BattleOutcome::Victory) | Some(BattleOutcome::Escaped) => self.continue_voyage(),
            Some(BattleOutcome::LiYuenRescue) => {
                if let Some(v) = self.voyage.as_mut() {
                    v.stage = VoyageStage::LiYuenForced;
                }
                self.continue_voyage();
            }
            Some(BattleOutcome::Lost) => {
                self.outcome = Some(scoring::end_game(
                    &self.state,
                    false,
                    "The buggers got us, Taipan!!! It's all over now!!!",
                ));
                self.mode = Mode::GameOver;
            }
            None => {
                // Shouldn't happen; restore and keep fighting.
                self.battle = Some(battle);
            }
        }
    }
}
