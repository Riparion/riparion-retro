//! The world actor: the single owner of the authoritative [`World`].
//!
//! All mutation of the shared market is serialized through this one task, so
//! there are no locks and no races — bots advance on a clock, and human trade
//! orders arrive as [`Command`]s on an mpsc channel. Market changes fan out to
//! every connected client over a broadcast channel.

use std::time::Duration;

use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::interval;

use tracing::{debug, info, warn};

use kaintuck_engine::market_link::{Side, TradeIntent};
use kaintuck_engine::net::{MarketSnapshot, ServerMsg};
use kaintuck_engine::sim::PolicyKind;
use kaintuck_engine::gossip::GossipKind;
use kaintuck_engine::scenario_data::scenario;
use kaintuck_engine::state::{GOOD_NAMES, NUM_GOODS, NUM_RIVER_TOWNS};
use kaintuck_engine::world::World;

/// Cap on gossip events fanned out per tick, so a busy world can't flood clients.
const MAX_GOSSIP_PER_TICK: usize = 6;

/// Upper bound on a single human trade order. A boat's hold is at most a few
/// hundred units, so this is generous for honest play while stopping one client
/// from slamming a price to the clamp or corrupting the volume metric with an
/// absurd quantity. (The full trust model — verifying the player can afford it —
/// is still TODO; this is the cheap guardrail.)
const MAX_ORDER_QTY: i64 = 1000;

/// A request to the world actor.
pub enum Command {
    /// A client joined; hand back its id and the current market.
    Join {
        reply: oneshot::Sender<(u64, MarketSnapshot)>,
    },
    /// A human trade order; reply with the fill price or an error.
    Trade {
        town: usize,
        good: usize,
        side: Side,
        qty: i64,
        reply: oneshot::Sender<Result<f64, String>>,
    },
    /// Resend the full market snapshot.
    Snapshot {
        reply: oneshot::Sender<MarketSnapshot>,
    },
}

/// A cloneable handle the connection tasks use to talk to the actor.
#[derive(Clone)]
pub struct Handle {
    pub cmd: mpsc::Sender<Command>,
    pub events: broadcast::Sender<ServerMsg>,
}

fn snapshot(world: &World) -> MarketSnapshot {
    MarketSnapshot {
        mids: (0..NUM_RIVER_TOWNS)
            .map(|t| world.market.snapshot_for(t).mid)
            .collect(),
    }
}

/// Build the world (with `bots` bot traders), spawn its actor task on a
/// `tick_ms` clock, and return a [`Handle`] for connections.
pub fn spawn(bots: usize, tick_ms: u64) -> Handle {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(256);
    let (evt_tx, _evt_rx0) = broadcast::channel::<ServerMsg>(2048);

    // Seed the world with a mix of bot strategies so the market sees both buy
    // and sell pressure plus a fuzzed tail; they keep it alive between humans.
    // Names come from the scenario's gossip pool so the crew can talk about them.
    let names = &scenario().gossip.names;
    let mut world = World::new();
    for i in 0..bots {
        let seed = 1 + i as u64;
        let kind = match i % 3 {
            0 => PolicyKind::Greedy,
            1 => PolicyKind::Cautious,
            _ => PolicyKind::Random,
        };
        let name = if names.is_empty() {
            format!("bot{i}")
        } else {
            // Round-robin the pool; once it wraps, suffix a numeral so two bots
            // never share a name (keeps the "recurring character" identity sane).
            let base = &names[i % names.len()];
            let wrap = i / names.len();
            if wrap == 0 {
                base.clone()
            } else {
                format!("{base} {}", wrap + 1)
            }
        };
        world.add_bot(seed, name, kind.make(i, seed));
    }
    info!(bots, tick_ms, "world spawned");

    let events = evt_tx.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(tick_ms.max(1)));
        let mut next_seed = 10_000u64;
        let mut next_human_id = 1_000_000u64;
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    world.tick(1.0);
                    let respawned = world.respawn_finished(&mut next_seed);
                    if respawned > 0 {
                        debug!(respawned, "bots finished a journey, respawned");
                    }
                    // Fan the whole market out each tick. With ~11 towns and a
                    // sub-second clock this is trivial bandwidth, and it keeps
                    // every client's quotes live without per-town diffing.
                    for town in 0..NUM_RIVER_TOWNS {
                        let mids = world.market.snapshot_for(town).mid;
                        // Ignore send errors (no subscribers yet / all lagged).
                        let _ = evt_tx.send(ServerMsg::MarketDelta { town, mids });
                    }
                    // Fan out attributed trader gossip (bot trades & milestones)
                    // so connected players' crews can talk about the other
                    // traders. Capped per tick to bound bandwidth.
                    let mut gossip = world.drain_gossip();
                    if !gossip.is_empty() {
                        // Keep high-signal milestones ahead of routine trades when
                        // capping, so a busy tick can't truncate away a
                        // Won/Lost/ReachedNatchez/Robbed (stable sort: false<true).
                        gossip.sort_by_key(|g| matches!(g.kind, GossipKind::Trade { .. }));
                        gossip.truncate(MAX_GOSSIP_PER_TICK);
                        for g in &gossip {
                            debug!(trader = %g.trader, ?g.persona, "trader gossip");
                        }
                        let _ = evt_tx.send(ServerMsg::Gossip(gossip));
                    }
                    // Periodic heartbeat at debug: market state under load.
                    if world.ticks % 100 == 0 {
                        debug!(
                            tick = world.ticks,
                            live_bots = world.live_bots(),
                            inflation = world.market.inflation(),
                            volume = world.market.total_volume(),
                            "tick"
                        );
                    }
                }
                maybe_cmd = cmd_rx.recv() => {
                    let Some(cmd) = maybe_cmd else {
                        info!("command channel closed — actor stopping");
                        break;
                    };
                    match cmd {
                        Command::Join { reply } => {
                            let id = next_human_id;
                            next_human_id += 1;
                            info!(participant_id = id, "human player joined");
                            let _ = reply.send((id, snapshot(&world)));
                        }
                        Command::Snapshot { reply } => {
                            debug!("client resync");
                            let _ = reply.send(snapshot(&world));
                        }
                        Command::Trade { town, good, side, qty, reply } => {
                            if town >= NUM_RIVER_TOWNS || good >= NUM_GOODS || qty <= 0 || qty > MAX_ORDER_QTY {
                                warn!(town, good, ?side, qty, "rejected invalid trade order");
                                let _ = reply.send(Err("invalid trade order".into()));
                            } else {
                                // Fill at the mid the player saw, then apply the
                                // pressure to the authoritative market.
                                let mid = world.market.quote(town, good);
                                world.apply_human_trade(&TradeIntent {
                                    town, good, side, qty, unit_price: mid,
                                });
                                info!(
                                    ?side,
                                    qty,
                                    good = GOOD_NAMES[good],
                                    town,
                                    mid,
                                    new_mid = world.market.quote(town, good),
                                    "human trade filled"
                                );
                                let _ = reply.send(Ok(mid));
                                // Push the moved town immediately so others see it.
                                let mids = world.market.snapshot_for(town).mid;
                                let _ = evt_tx.send(ServerMsg::MarketDelta { town, mids });
                            }
                        }
                    }
                }
            }
        }
    });

    Handle { cmd: cmd_tx, events }
}
