//! The automatic phase bodies — the testable heart of the port. Each function
//! mirrors one routine of `imports/dukedom.c`, preserving its exact formulas and
//! C integer-truncation semantics (`as i64` truncates toward zero, like a C cast;
//! `i64 / i64` truncates toward zero, like C integer division).

use super::scoring;
use super::state::{Ending, M, RETIREMENT_YEAR};
use super::{EndGame, Flow, Game, Interaction, Phase, WarCtx};

/// The deposition narration, shared by the start-of-year unrest gate and the
/// starvation path so the two can't drift.
const CAUSE_DEPOSED: &str = "The peasants tire of war and starvation. You are deposed.";

impl Game {
    /// The lose/win gate (BASIC `end_of_game_check`, lines 368-393). The
    /// double-tax demand and `U1` reset live in the driver's phase arm.
    pub(crate) fn check_end_of_game(&self) -> Option<EndGame> {
        let s = &self.state;
        if s.n_p < 33 {
            return Some(scoring::build_end(
                s,
                Ending::Collapsed,
                "So few peasants remain that the High King has abolished your Ducal right.".into(),
            ));
        }
        if s.n_l < 199 {
            return Some(scoring::build_end(
                s,
                Ending::Collapsed,
                "So little land remains that the High King has abolished your Ducal right.".into(),
            ));
        }
        if s.n_g < 429 || s.u1 > 88 || s.u2 > 99 {
            return Some(scoring::build_end(
                s,
                Ending::Deposed,
                CAUSE_DEPOSED.into(),
            ));
        }
        if s.n_y > RETIREMENT_YEAR && s.k == 0 {
            return Some(scoring::build_end(
                s,
                Ending::Retired,
                "You have reached the age of mandatory retirement, your duchy intact.".into(),
            ));
        }
        None
    }

    /// Winter starvation and the unrest it breeds (BASIC `starvation_and_unrest`).
    pub(crate) fn run_starvation(&mut self) {
        let per_capita = (self.fed_grain / self.state.n_p.max(1)) as f64;
        let mut x1 = per_capita;
        if x1 < 13.0 {
            self.message("Some peasants starved during the long winter.");
            let survivors = self.fed_grain / 13;
            self.state.p[2] = -(self.state.n_p - survivors);
            self.state.n_p += self.state.p[2];
        }
        x1 -= 14.0;
        if x1 > 4.0 {
            x1 = 4.0;
        }
        self.state.u1 = self.state.u1 - 3 * self.state.p[2] - (2.0 * x1) as i64;
        if self.state.u1 > 88 {
            let end = scoring::build_end(
                &self.state,
                Ending::Deposed,
                CAUSE_DEPOSED.into(),
            );
            self.set_outcome(end);
            return;
        }
        if self.state.n_p < 33 {
            if let Some(end) = self.check_end_of_game() {
                self.set_outcome(end);
            }
        }
    }

    /// Quote this year's land prices (BASIC `purchase_land`, lines 470-473). The
    /// buy price draws `FNX(1)` once; the sell price is the best standing offer.
    pub(crate) fn prepare_land_prices(&mut self) {
        let c = self.state.c1;
        let mut buy = (2.0 * c + self.fnx(1) as f64 - 5.0) as i64;
        if buy < 4 {
            buy = 4;
        }
        self.land_buy_price = buy;
        self.input_error = None;
    }

    /// Apply the King's peasant levy (BASIC `crop_yield_and_losses`, lines 721-728).
    pub(crate) fn apply_king_levy(&mut self, peasants: i64, grain: i64, supply: bool) {
        if supply {
            self.state.p[3] = -peasants;
            self.state.n_p += self.state.p[3];
        } else {
            self.state.g[10] = -grain;
            self.state.n_g += self.state.g[10];
        }
    }

    /// The King's army attacks (BASIC `war_with_the_king`, lines 575-598). Win and
    /// you seize the crown; lose and you are beheaded.
    pub(crate) fn run_war_with_king(&mut self) {
        let mercenaries = self.state.n_g as f64 / 100.0;
        self.message(format!(
            "The King's army attacks! You spend all your grain to hire {mercenaries:.0} foreign mercenaries at 100 HL each."
        ));
        if self.state.n_g as f64 * mercenaries + self.state.n_p as f64 > 2399.0 {
            let end = scoring::build_end(
                &self.state,
                Ending::HighKing,
                "Wipe the blood from the crown — you are now High King!".into(),
            );
            self.set_outcome(end);
        } else {
            let end = scoring::build_end(
                &self.state,
                Ending::Beheaded,
                "Your head atop the castle gate signifies the High King has abolished your Ducal right.".into(),
            );
            self.set_outcome(end);
        }
    }

    /// Degrade cropped soil and recover fallow land (BASIC `update_land_tables`).
    /// The exact index arithmetic is load-bearing — the loops walk `s` and `u` at
    /// offset indices, so plain indexing is clearer than any iterator rewrite.
    #[allow(clippy::needless_range_loop)]
    pub(crate) fn run_update_land_tables(&mut self) {
        let mut v = self.state.g[8]; // acres planted
        let mut u = [0i64; 7];
        // Consume cropped acres from the best tiers down.
        let mut idx = 6;
        for j1 in 1..=6 {
            if v <= self.state.s[j1] {
                idx = j1;
                break;
            }
            v -= self.state.s[j1];
            u[j1] = self.state.s[j1];
            self.state.s[j1] = 0;
        }
        u[idx] = v;
        self.state.s[idx] -= v;
        // Fallow land improves (moves up two tiers); cropped land degrades one.
        self.state.s[1] += self.state.s[2];
        self.state.s[2] = 0;
        for j1 in 3..=6 {
            self.state.s[j1 - 2] += self.state.s[j1];
            self.state.s[j1] = 0;
        }
        for j1 in 1..=5 {
            self.state.s[j1 + 1] += u[j1];
        }
        self.state.s[6] += u[6];
        self.state.u = u;
    }

    /// Harvest yield, rat losses, and the King's peasant levy (BASIC
    /// `crop_yield_and_losses`). Returns [`Flow::Pause`] if a levy decision is
    /// queued. Note: as in the original, the levy only arises when rats do.
    pub(crate) fn run_crop_yield(&mut self) -> Flow {
        let mut c = self.fnx(2) as f64 + 3.0;
        if (self.state.n_y / 7) * 7 == self.state.n_y {
            self.message("Seven-year locusts strip the fields!");
            c /= 2.0;
        }
        // Fertility-weighted area of the cropped land (weights 1.0 … 0.2).
        let mut weighted = 0.0;
        for j1 in 1..=5 {
            weighted += self.state.u[j1] as f64 * (1.2 - 0.2 * j1 as f64);
        }
        let g8 = self.state.g[8];
        if g8 == 0 {
            self.state.c1 = 0.0;
            c = 0.0;
        } else {
            let c1 = (c * ((weighted / g8 as f64) * 100.0) / 100.0) as i64;
            self.state.c1 = c1 as f64;
            c = c1 as f64;
        }
        self.state.c = c;
        self.message(format!("The harvest yields {c:.0} HL per HA."));

        // Rats — and only if rats strike does the King consider a levy.
        let rats = self.fnx(3) as f64 + 3.0;
        if rats < 9.0 {
            return Flow::Continue;
        }
        self.state.g[5] = -((rats * self.state.n_g as f64) / 83.0) as i64;
        self.state.n_g += self.state.g[5];
        self.message("Rats infest the granary!");
        if self.state.n_p < 67 || self.state.k == -1 {
            return Flow::Continue;
        }
        let levy = self.fnx(4);
        if levy as f64 > self.state.n_p as f64 / 30.0 {
            return Flow::Continue;
        }
        let peasants = levy;
        let grain = levy * 100;
        self.message(format!(
            "The High King requires {peasants} peasants for his estates and mines."
        ));
        self.pending
            .push_back(Interaction::KingLevy { peasants, grain });
        Flow::Pause
    }

    /// A rival Duke may threaten war (BASIC `war`, lines 733-762). Sets the year
    /// cursor to [`Phase::Population`] itself: the battle's sub-decisions resolve
    /// through the queue (`resolve_attack_choice` / `resolve_mercenaries`), so the
    /// driver never re-enters this routine mid-war.
    pub(crate) fn run_war(&mut self) -> Flow {
        self.phase = Phase::Population;

        if self.state.k == -1 {
            self.message("The High King calls for peasant levies and hires foreign mercenaries.");
            self.state.k = -2;
            return Flow::Continue;
        }

        let mut threshold = (11.0 - 1.5 * self.state.c) as i64;
        if threshold < 2 {
            threshold = 2;
        }
        let mut enemy_seed = 0;
        if self.state.k == 0
            && self.state.n_p > 109
            && (17 * (self.state.n_l - 400) + self.state.n_g) > 10600
        {
            self.message("The High King grows uneasy and may be subsidizing wars against you.");
            threshold += 2;
            enemy_seed = self.state.n_y + 5;
        }
        let x3 = self.fnx(5);
        if x3 > threshold {
            return Flow::Continue; // no war this year
        }
        let enemy = enemy_seed + 85 + 18 * self.fnx(6);
        let troop_mult = (1.2 - (self.state.u1 / 16) as f64) as i64;
        self.war = Some(WarCtx {
            enemy_strength: enemy,
            troop_mult,
            threshold,
            defense_x3: x3,
        });
        self.message("A nearby Duke threatens war!");
        self.pending.push_back(Interaction::WarAttack);
        Flow::Pause
    }

    /// Resolve the attack-first decision (BASIC `war`, lines 767-795).
    pub(crate) fn resolve_attack_choice(&mut self, attack: bool) {
        let Some(ctx) = self.war else {
            self.phase = Phase::Population;
            return;
        };
        if !attack {
            self.pending
                .push_back(Interaction::WarMercenary { max: 75, price: 40 });
            return;
        }
        let x5 = self.state.n_p * ctx.troop_mult + 13;
        if ctx.enemy_strength >= x5 {
            self.message("Your first strike failed — you need professionals.");
            let p4 = -ctx.defense_x3 - ctx.threshold - 2;
            self.state.p[4] = p4;
            self.state.n_p += p4;
            let new_enemy = ctx.enemy_strength + 3 * p4;
            if new_enemy < 1 {
                self.end_war_no_battle();
                return;
            }
            self.war = Some(WarCtx {
                enemy_strength: new_enemy,
                ..ctx
            });
            self.pending
                .push_back(Interaction::WarMercenary { max: 75, price: 40 });
        } else {
            self.message("Peace negotiations were successful.");
            let p4 = -ctx.threshold - 1;
            self.state.p[4] = p4;
            self.state.n_p += p4;
            self.end_war_no_battle();
        }
    }

    /// A war ended without pitched battle (negotiated or averted by a first strike).
    fn end_war_no_battle(&mut self) {
        self.state.u1 = self.state.u1 - 2 * self.state.p[4] - 3 * self.state.p[5];
        self.war = None;
    }

    /// Resolve the pitched battle after mercenary hire (BASIC `war`, lines 796-909).
    pub(crate) fn resolve_mercenaries(&mut self, hired: i64) {
        let v = hired.clamp(0, 75);
        let Some(ctx) = self.war else {
            self.phase = Phase::Population;
            return;
        };
        let enemy = (ctx.enemy_strength as f64 * M) as i64;
        let x5 = (self.state.n_p as f64 * ctx.troop_mult as f64 + 7.0 * v as f64 + 13.0) as i64;
        let mut x6 = enemy - 4 * v - (0.25 * x5 as f64) as i64;
        let margin = x5 - enemy; // > 0 means you prevail
        let l3 = (0.8 * margin as f64) as i64;
        self.state.l[3] = l3;

        if -l3 > (0.67 * self.state.n_l as f64) as i64 {
            let end = scoring::build_end(
                &self.state,
                Ending::Beheaded,
                "You are overrun and lose the entire Dukedom. Your head adorns the castle gate."
                    .into(),
            );
            self.set_outcome(end);
            self.war = None;
            return;
        }

        // Distribute the land swing across the fertility tiers.
        let mut rem = l3 as f64;
        for j1 in 1..=3 {
            let x3 = (rem / (4 - j1) as f64) as i64;
            let take = if -x3 <= self.state.s[j1] {
                x3
            } else {
                -self.state.s[j1]
            };
            self.state.s[j1] += take;
            rem -= take as f64;
        }
        for j1 in 4..=6 {
            let whole = rem as i64;
            let take = if -rem <= self.state.s[j1] as f64 {
                whole
            } else {
                -self.state.s[j1]
            };
            self.state.s[j1] += take;
            rem -= take as f64;
        }

        // X4 is reused as an int factor here, so 0.67 / 0.55 truncate to 0.
        let x4_factor: i64;
        if l3 < 399 {
            if margin >= 0 {
                self.message("You have won the war!");
                x4_factor = 0;
                self.state.g[7] = (1.7 * l3 as f64) as i64;
                self.state.n_g += self.state.g[7];
            } else {
                self.message("You have lost the war.");
                x4_factor = self.state.g[8] / self.state.n_l.max(1);
            }
            if x6 <= 9 {
                x6 = 0;
            } else {
                x6 /= 10;
            }
        } else {
            self.message("You overrun the enemy and annex his entire Dukedom!");
            self.state.g[7] = 3513;
            self.state.n_g += 3513;
            x6 = -47;
            x4_factor = 0;
            if self.state.k <= 0 {
                self.state.k = 1;
                self.message("The King fears for his throne and may be planning direct action.");
            }
        }

        if x6 > self.state.n_p {
            x6 = self.state.n_p;
        }
        self.state.p[4] -= x6;
        self.state.n_p -= x6;
        self.state.g[8] += x4_factor * l3;
        let cost = 40 * v;
        if cost <= self.state.n_g {
            self.state.g[6] = -cost;
        } else {
            self.state.g[6] = -self.state.n_g;
            self.state.p[5] = -((cost - self.state.n_g) / 7) - 1;
            self.message("There isn't enough grain to pay the mercenaries!");
        }
        self.state.n_g += self.state.g[6];
        self.state.n_p += self.state.p[5];
        self.state.n_l += l3;
        self.state.u1 = self.state.u1 - 2 * self.state.p[4] - 3 * self.state.p[5];
        self.war = None;
    }

    /// Plague, births, and natural deaths (BASIC `population_changes`).
    pub(crate) fn run_population(&mut self) {
        let roll = self.fnx(7);
        if roll <= 3 {
            if roll != 1 {
                self.message("A POX EPIDEMIC breaks out!");
                let divisor = (roll * 5).max(1);
                self.state.p[6] = -(self.state.n_p / divisor);
                self.state.n_p += self.state.p[6];
            } else if self.state.d <= 0 {
                self.message("The BLACK PLAGUE strikes the land!");
                self.state.d = 13;
                self.state.p[6] = -(self.state.n_p / 3);
                self.state.n_p += self.state.p[6];
            }
        }
        // Births and natural deaths (always).
        let mut divisor = (self.fnx(8) + 4) as f64;
        if self.state.p[5] != 0 {
            divisor = 4.5;
        }
        if divisor.abs() < 1.0 {
            divisor = 4.0;
        }
        self.state.p[8] = (self.state.n_p as f64 / divisor) as i64;
        self.state.p[7] = (0.3 - (self.state.n_p / 22) as f64) as i64;
        self.state.n_p += self.state.p[7] + self.state.p[8];
        self.state.d -= 1;
    }

    /// Bank the harvest, pay the castle and the royal tax (BASIC `harvest_grain`).
    pub(crate) fn run_harvest(&mut self) {
        self.state.g[8] = (self.state.c * self.state.g[8] as f64) as i64;
        self.state.n_g += self.state.g[8];
        let over = self.state.g[8] - 4000;
        if over > 0 {
            self.state.g[9] = -((0.1 * over as f64) as i64);
        }
        self.state.g[9] -= 120;
        self.state.n_g += self.state.g[9];

        if self.state.k < 0 {
            return; // at war with the King — no tax collected
        }
        let mut tax = -(self.state.n_l / 2);
        if self.state.k >= 2 {
            tax *= 2;
        }
        if -tax > self.state.n_g {
            self.message("There is not enough grain to pay the royal tax.");
            let end = scoring::build_end(
                &self.state,
                Ending::Collapsed,
                "You cannot pay the royal tax. The High King abolishes your Ducal right.".into(),
            );
            self.set_outcome(end);
            return;
        }
        self.state.g[10] += tax;
        self.state.n_g += tax;
    }

    /// Decay cumulative unrest, then fold in this year's (BASIC `update_unrest`).
    pub(crate) fn run_update_unrest(&mut self) {
        self.state.u2 = (self.state.u2 as f64 * 0.85) as i64 + self.state.u1;
    }
}
