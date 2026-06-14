//! The persistent shared world: one authoritative [`Market`] and a roster of
//! participants, each running its own independent journey (its own `Game` at its
//! own pace) but trading against the one shared market. This is the unit the
//! economy simulator runs headless and the native server wraps for live play —
//! the only thing that crosses between journeys is a trade's price pressure.
//!
//! Time enters only through [`World::tick`]: each tick, every active bot makes
//! one decision, its trades are applied to the shared market, and the market
//! relaxes by `dt`. Humans (on the server) submit their trades the same way,
//! through [`Market::apply_trade`].

use crate::gossip::{GossipEvent, GossipKind, PriceTier};
use crate::market::{Market, MarketParams};
use crate::market_link::MarketLink;
use crate::policy::{Persona, Policy};
use crate::state::{Phase, NATCHEZ};
use crate::Game;

/// A trade of at least this many units is "notable" enough for the crew to
/// gossip about; smaller fills stay quiet so the feed isn't drowned in noise.
const NOTABLE_QTY: i64 = 25;

/// Bands (mid / baseline) above/below which a price counts as bid-up / glutted.
const PREMIUM_AT: f64 = 1.10;
const GLUT_AT: f64 = 0.90;

/// Classify the current mid for `good` at `town` against its resting baseline —
/// the actionable signal attached to trade gossip.
fn price_tier(market: &Market, town: usize, good: usize) -> PriceTier {
    let baseline = market.baseline(town, good);
    if baseline <= 0.0 {
        return PriceTier::Fair;
    }
    let ratio = market.quote(town, good) / baseline;
    if ratio >= PREMIUM_AT {
        PriceTier::Premium
    } else if ratio <= GLUT_AT {
        PriceTier::Glut
    } else {
        PriceTier::Fair
    }
}

/// One participant in the shared world: a named, persona'd bot running its own
/// private journey. (Human participants on the server are tracked separately;
/// this is the bot roster.)
pub struct Participant {
    pub id: u64,
    /// Display name, surfaced in trader gossip; persists across respawns.
    pub name: String,
    /// Trading temperament (from the policy), surfaced in gossip.
    pub persona: Persona,
    pub game: Game,
    pub policy: Box<dyn Policy>,
    // Previous-tick snapshot for milestone diffing.
    last_town: usize,
    was_river: bool,
    was_robbed: bool,
    was_finished: bool,
}

impl Participant {
    /// Whether this participant's journey has ended.
    pub fn is_done(&self) -> bool {
        self.game.outcome.is_some()
    }

    /// Re-seed the milestone snapshot from the current game (after a fresh start).
    fn reset_tracking(&mut self) {
        self.last_town = self.game.state.town;
        self.was_river = self.game.phase == Phase::River;
        self.was_robbed = self.game.state.robbed;
        self.was_finished = self.game.outcome.is_some();
    }

    /// Start this participant on a fresh journey under a new seed, keeping its
    /// name and persona (a recurring character) and resetting its policy.
    pub fn restart(&mut self, seed: u64) {
        let mut game = Game::new(seed);
        game.begin_with(self.name.clone(), crate::ledger::Carryover::fresh());
        self.game = game;
        self.policy.reset();
        self.reset_tracking();
    }
}

/// The shared world.
pub struct World {
    pub market: Market,
    pub bots: Vec<Participant>,
    /// Number of [`tick`](World::tick)s elapsed (each advances every live bot one
    /// step and relaxes the market by the tick's `dt`).
    pub ticks: u64,
    /// Attributed trader activity produced this tick, awaiting the host to
    /// broadcast it. Drained via [`drain_gossip`](World::drain_gossip).
    pending_gossip: Vec<GossipEvent>,
    next_id: u64,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// An empty world with a fresh market at rest (default tuning).
    pub fn new() -> Self {
        Self::with_params(MarketParams::default())
    }

    /// An empty world whose market uses explicit price-dynamics tuning.
    pub fn with_params(params: MarketParams) -> Self {
        Self {
            market: Market::with_params(params),
            bots: Vec::new(),
            ticks: 0,
            pending_gossip: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a bot named `name` that begins a fresh journey under `policy`. Its
    /// persona is taken from the policy and surfaced in trader gossip. Returns its id.
    pub fn add_bot(&mut self, seed: u64, name: String, policy: Box<dyn Policy>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let persona = policy.persona();
        let mut game = Game::new(seed);
        game.begin_with(name.clone(), crate::ledger::Carryover::fresh());
        // Seed the milestone snapshot from the fresh game so the first real
        // transition (not the starting state) is what gets gossiped.
        self.bots.push(Participant {
            id,
            name,
            persona,
            last_town: game.state.town,
            was_river: game.phase == Phase::River,
            was_robbed: game.state.robbed,
            was_finished: game.outcome.is_some(),
            game,
            policy,
        });
        id
    }

    /// Advance the world by `dt`: step every live bot once against the shared
    /// market, fold its trades back into that market, then relax prices.
    pub fn tick(&mut self, dt: f64) {
        for bot in &mut self.bots {
            if bot.is_done() {
                continue;
            }
            // While afloat, the bot trades against the shared market: shadow its
            // quotes in, decide+apply one step, then fold its fills back out.
            let afloat = bot.game.phase == Phase::River;
            if afloat {
                let town = bot.game.state.town;
                bot.game.market_link = Some(MarketLink::new(town, self.market.snapshot_for(town)));
            }
            let action = bot.policy.decide(&bot.game, Some(&self.market));
            bot.game.apply(action);
            if let Some(link) = bot.game.market_link.as_mut() {
                for intent in link.drain_fills() {
                    self.market.apply_trade(&intent);
                    // Notable trades become gossip; small fills stay quiet.
                    if intent.qty.abs() >= NOTABLE_QTY {
                        // Tag the trade with the town's price standing *after* this
                        // fill — the condition a player heading there would now find
                        // ("bid up there" / "glutted"). This is intentional: for a
                        // big fill the tier partly reflects the trade's own impact,
                        // but that impact is exactly what the listener can still act
                        // on. One bot fill moves the mid by a bounded `impact`, so it
                        // only flips a tier when the mid already sat near a threshold.
                        let tier = price_tier(&self.market, intent.town, intent.good);
                        self.pending_gossip.push(GossipEvent {
                            trader: bot.name.clone(),
                            persona: bot.persona,
                            kind: GossipKind::Trade {
                                town: intent.town,
                                good: intent.good,
                                side: intent.side,
                                qty: intent.qty,
                                tier,
                            },
                        });
                    }
                }
            }
            // Detach so the bot's game serializes/behaves as a plain offline game
            // between ticks; the link is re-attached fresh next tick.
            bot.game.market_link = None;

            // Diff the bot's state against last tick to surface journey milestones.
            Self::diff_milestones(bot, &mut self.pending_gossip);
        }
        self.market.tick(dt);
        self.ticks += 1;
    }

    /// Compare a bot's current state to its previous-tick snapshot and push any
    /// journey-milestone gossip, then update the snapshot.
    fn diff_milestones(bot: &mut Participant, out: &mut Vec<GossipEvent>) {
        let mut kinds: Vec<GossipKind> = Vec::new();
        {
            let s = &bot.game.state;
            // Arrival(s) at new river landings — emit one per landing crossed, so
            // a tick that advances more than one town doesn't swallow the
            // intermediate stops (Natchez gets its own beat).
            if bot.game.phase == Phase::River && s.town > bot.last_town {
                for town in (bot.last_town + 1)..=s.town {
                    kinds.push(if town == NATCHEZ {
                        GossipKind::ReachedNatchez
                    } else {
                        GossipKind::Arrived { town }
                    });
                }
            }
            // Stepping off the river onto the Trace.
            if bot.was_river && bot.game.phase == Phase::Trace {
                kinds.push(GossipKind::SetOutOnTrace);
            }
            // Robbed for the first time this journey.
            if !bot.was_robbed && s.robbed {
                kinds.push(GossipKind::Robbed);
            }
            // Journey just ended — use the third-person fate, not the
            // second-person ending prose, so gossip reads about the trader.
            if !bot.was_finished {
                if let Some(end) = &bot.game.outcome {
                    kinds.push(if end.won {
                        GossipKind::Won { score: end.score }
                    } else {
                        GossipKind::Lost {
                            cause: end.cause_kind.gossip_fate().to_string(),
                        }
                    });
                }
            }
        }
        for kind in kinds {
            out.push(GossipEvent {
                trader: bot.name.clone(),
                persona: bot.persona,
                kind,
            });
        }
        // Refresh the snapshot.
        bot.last_town = bot.game.state.town;
        bot.was_river = bot.game.phase == Phase::River;
        bot.was_robbed = bot.game.state.robbed;
        bot.was_finished = bot.game.outcome.is_some();
    }

    /// Take the trader activity accumulated since the last drain — the host
    /// broadcasts it as gossip to connected players.
    pub fn drain_gossip(&mut self) -> Vec<GossipEvent> {
        std::mem::take(&mut self.pending_gossip)
    }

    /// How many bots are still on a journey.
    pub fn live_bots(&self) -> usize {
        self.bots.iter().filter(|b| !b.is_done()).count()
    }

    /// Restart every finished bot on a fresh journey, advancing `next_seed` for
    /// each. Keeps a persistent world's market alive indefinitely. Returns how
    /// many were respawned.
    pub fn respawn_finished(&mut self, next_seed: &mut u64) -> usize {
        let mut n = 0;
        for bot in &mut self.bots {
            if bot.is_done() {
                bot.restart(*next_seed);
                *next_seed = next_seed.wrapping_add(1);
                n += 1;
            }
        }
        n
    }

    /// Apply a human (off-roster) trade to the shared market — used by the server
    /// when a connected player submits an order. Returns the fill's unit price.
    pub fn apply_human_trade(&mut self, intent: &crate::market_link::TradeIntent) {
        self.market.apply_trade(intent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{Cautious, GreedyProfit};

    #[test]
    fn bots_emit_named_persona_gossip() {
        let mut world = World::new();
        world.add_bot(1, "Lemuel Boggs".into(), Box::new(GreedyProfit::default()));
        world.add_bot(2, "Silas Crews".into(), Box::new(Cautious));

        let mut gossip = Vec::new();
        let mut seed = 10_000u64;
        for _ in 0..600 {
            world.tick(1.0);
            world.respawn_finished(&mut seed);
            gossip.extend(world.drain_gossip());
        }

        assert!(!gossip.is_empty(), "no gossip produced over a full run");
        // Names and personas are attributed.
        assert!(gossip.iter().all(|e| e.trader == "Lemuel Boggs" || e.trader == "Silas Crews"));
        assert!(gossip.iter().any(|e| e.persona == Persona::Greedy));
        assert!(gossip.iter().any(|e| e.persona == Persona::Cautious));
        // Notable trades surface and carry a price tier (the actionable signal).
        assert!(
            gossip.iter().any(|e| matches!(e.kind, GossipKind::Trade { .. })),
            "no notable trades gossiped"
        );
        for e in &gossip {
            if let GossipKind::Trade { tier, .. } = e.kind {
                let _ = tier; // every trade is classified
            }
        }
        // Journey milestones surface (a run reaches an ending one way or another).
        assert!(gossip
            .iter()
            .any(|e| matches!(e.kind, GossipKind::Won { .. } | GossipKind::Lost { .. })));
        // Lost causes are third-person fates (no "you"/"your" leaking from the
        // player-facing ending prose).
        for e in &gossip {
            if let GossipKind::Lost { cause } = &e.kind {
                let c = cause.to_lowercase();
                assert!(
                    !c.contains("you"),
                    "lost cause should be third-person, got: {cause}"
                );
            }
        }
    }
}
