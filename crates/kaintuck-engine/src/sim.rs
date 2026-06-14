//! The autonomous economy simulator: spin up a [`World`] of bot traders, run it
//! for a number of ticks against the shared [`Market`](crate::market::Market),
//! and collect metrics — price trajectories, traded volume, inflation, and how
//! the bots fared. This is the headless "economy under load" harness, used for
//! balance tuning and (via the CLI) throughput benchmarking.
//!
//! Everything here is pure and deterministic for a given config: same seeds and
//! tick count → same report. Wall-clock throughput is measured by the caller
//! (the native CLI), since the engine has no clock.

use serde::Serialize;

use crate::market::MarketParams;
use crate::policy::{Cautious, GreedyProfit, Policy, RandomFuzz};
use crate::state::{GOOD_NAMES, NUM_GOODS, NUM_RIVER_TOWNS};
use crate::world::World;

/// Which bot strategy the simulated traders use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PolicyKind {
    Greedy,
    Cautious,
    Random,
    /// A mix: round-robin Greedy / Cautious / Random across the bots, so the
    /// market sees both buy and sell pressure and a fuzzed tail.
    Mixed,
}

impl PolicyKind {
    /// Build the policy for bot index `i` under this kind. `seed` feeds the fuzzer.
    pub fn make(self, i: usize, seed: u64) -> Box<dyn Policy> {
        match self {
            PolicyKind::Greedy => Box::new(GreedyProfit::default()),
            PolicyKind::Cautious => Box::new(Cautious),
            PolicyKind::Random => Box::new(RandomFuzz::new(seed)),
            PolicyKind::Mixed => match i % 3 {
                0 => Box::new(GreedyProfit::default()),
                1 => Box::new(Cautious),
                _ => Box::new(RandomFuzz::new(seed)),
            },
        }
    }

    /// Parse a CLI policy name.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "greedy" => Some(PolicyKind::Greedy),
            "cautious" => Some(PolicyKind::Cautious),
            "random" | "fuzz" => Some(PolicyKind::Random),
            "mixed" | "mix" => Some(PolicyKind::Mixed),
            _ => None,
        }
    }
}

/// What to run.
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Number of bot traders sharing the market.
    pub bots: usize,
    /// Base seed; bot `i` is seeded `base_seed + i`.
    pub base_seed: u64,
    /// How many ticks to run (each tick = one decision per live bot + relaxation).
    pub ticks: u64,
    /// Time advanced per tick (drives market relaxation).
    pub dt: f64,
    /// Bot strategy.
    pub policy: PolicyKind,
    /// Market price-dynamics tuning to run with.
    pub params: MarketParams,
    /// Respawn finished bots so the market stays under continuous load — models
    /// the persistent server. When false, bots run a single journey and the
    /// market relaxes once they finish (a burst-then-decay run).
    pub sustained: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            bots: 8,
            base_seed: 1,
            ticks: 2000,
            dt: 1.0,
            policy: PolicyKind::Mixed,
            params: MarketParams::default(),
            sustained: true,
        }
    }
}

/// The final mid + baseline + traded volume for one good at one town.
#[derive(Debug, Clone, Serialize)]
pub struct GoodStat {
    pub town: usize,
    pub good: usize,
    pub good_name: String,
    pub baseline: f64,
    pub final_mid: f64,
    /// `final_mid / baseline` — >1 bid up, <1 sold down.
    pub ratio: f64,
    pub volume: f64,
}

/// How a finished bot fared.
#[derive(Debug, Clone, Serialize)]
pub struct BotOutcome {
    pub id: u64,
    pub won: bool,
    pub score: i64,
    pub cause: String,
    pub cash: i64,
    pub days: i64,
}

/// The result of a simulation run.
#[derive(Debug, Clone, Serialize)]
pub struct SimReport {
    pub bots: usize,
    pub ticks: u64,
    pub dt: f64,
    pub policy: PolicyKind,
    pub sustained: bool,
    /// The market tuning this run used.
    pub relax_rate: f64,
    pub impact: f64,
    pub depth_units: f64,
    /// Journeys completed (sustained: cumulative across respawns).
    pub wins: usize,
    pub losses: usize,
    pub avg_score: f64,
    /// Mean mid/baseline ratio across the whole market at the end.
    pub final_inflation: f64,
    /// Mean inflation over the back half of the run — the steady-state level the
    /// market settles to under load.
    pub steady_inflation: f64,
    /// Highest / lowest whole-market inflation seen during the run.
    pub peak_inflation: f64,
    pub trough_inflation: f64,
    /// Std-dev of inflation over the back half — how lively (vs. pinned) it is.
    pub steady_volatility: f64,
    /// THE headline tuning number: steady-state mean absolute price deviation
    /// from baseline (back-half average). How much prices actually move under
    /// load — buy/sell pressure does NOT cancel here, unlike inflation.
    pub steady_dispersion: f64,
    /// The most-displaced single mid, peak over the run, as a fraction.
    pub peak_max_dispersion: f64,
    /// Count of town×good cells sitting near the price clamp (ratio < 0.3 or
    /// > 3.5) at the end — a sign of runaway/unstable tuning.
    pub near_clamp: usize,
    pub total_volume: f64,
    /// Per-tick market inflation samples (one per tick), for plotting trajectories.
    pub inflation_trace: Vec<f64>,
    /// Final per-town/good detail.
    pub goods: Vec<GoodStat>,
    pub outcomes: Vec<BotOutcome>,
}

impl SimReport {
    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("SimReport serializes")
    }

    /// A flat CSV of the per-town/good final stats (the most analysis-friendly
    /// slice). One header row plus one row per town×good.
    pub fn to_csv(&self) -> String {
        let mut out = String::from("town,good,good_name,baseline,final_mid,ratio,volume\n");
        for g in &self.goods {
            out.push_str(&format!(
                "{},{},{},{:.4},{:.4},{:.4},{:.1}\n",
                g.town, g.good, g.good_name, g.baseline, g.final_mid, g.ratio, g.volume
            ));
        }
        out
    }
}

/// Run a simulation and return its report.
pub fn run_sim(cfg: &SimConfig) -> SimReport {
    let mut world = World::with_params(cfg.params);
    for i in 0..cfg.bots {
        let seed = cfg.base_seed.wrapping_add(i as u64);
        let policy = cfg.policy.make(i, seed);
        world.add_bot(seed, format!("bot{i}"), policy);
    }
    let mut next_seed = cfg.base_seed.wrapping_add(cfg.bots as u64).wrapping_add(1);

    // Completed journeys, tallied as they finish (so respawns don't lose them).
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut score_sum = 0i64;

    let mut inflation_trace = Vec::with_capacity(cfg.ticks as usize);
    let mut dispersion_trace = Vec::with_capacity(cfg.ticks as usize);
    let mut peak_max_dispersion = 0.0_f64;
    for _ in 0..cfg.ticks {
        world.tick(cfg.dt);
        if cfg.sustained {
            // Tally anyone who just finished, then restart them so the market
            // stays under continuous load (as the live server does).
            for bot in &world.bots {
                if let Some(end) = &bot.game.outcome {
                    if end.won {
                        wins += 1;
                    } else {
                        losses += 1;
                    }
                    score_sum += end.score;
                }
            }
            world.respawn_finished(&mut next_seed);
        }
        inflation_trace.push(world.market.inflation());
        dispersion_trace.push(world.market.dispersion());
        peak_max_dispersion = peak_max_dispersion.max(world.market.max_dispersion());
    }

    // Per-town/good final detail.
    let mut goods = Vec::with_capacity(NUM_RIVER_TOWNS * NUM_GOODS);
    let mut near_clamp = 0usize;
    for town in 0..NUM_RIVER_TOWNS {
        for good in 0..NUM_GOODS {
            let baseline = world.market.baseline(town, good);
            let final_mid = world.market.quote(town, good);
            let ratio = if baseline > 0.0 { final_mid / baseline } else { 1.0 };
            if ratio < 0.3 || ratio > 3.5 {
                near_clamp += 1;
            }
            goods.push(GoodStat {
                town,
                good,
                good_name: GOOD_NAMES[good].to_string(),
                baseline,
                final_mid,
                ratio,
                volume: world.market.volume(town, good),
            });
        }
    }

    // In burst mode (no respawn), tally from the final bot states instead.
    let mut outcomes = Vec::new();
    if !cfg.sustained {
        for bot in &world.bots {
            if let Some(end) = &bot.game.outcome {
                if end.won {
                    wins += 1;
                } else {
                    losses += 1;
                }
                score_sum += end.score;
                outcomes.push(BotOutcome {
                    id: bot.id,
                    won: end.won,
                    score: end.score,
                    cause: end.cause.clone(),
                    cash: end.cash,
                    days: end.days,
                });
            }
        }
    }
    let finished = (wins + losses).max(1);
    let avg_score = score_sum as f64 / finished as f64;

    // Steady-state stats over the back half of the inflation trace.
    let half = inflation_trace.len() / 2;
    let cut = half.min(inflation_trace.len().saturating_sub(1));
    let tail = &inflation_trace[cut..];
    let steady_inflation = mean(tail);
    let steady_volatility = std_dev(tail, steady_inflation);
    let peak_inflation = inflation_trace.iter().copied().fold(f64::MIN, f64::max);
    let trough_inflation = inflation_trace.iter().copied().fold(f64::MAX, f64::min);
    let steady_dispersion = mean(&dispersion_trace[cut..]);

    SimReport {
        bots: cfg.bots,
        ticks: world.ticks,
        dt: cfg.dt,
        policy: cfg.policy,
        sustained: cfg.sustained,
        relax_rate: cfg.params.relax_rate,
        impact: cfg.params.impact,
        depth_units: cfg.params.depth_units,
        wins,
        losses,
        avg_score,
        final_inflation: world.market.inflation(),
        steady_inflation,
        peak_inflation,
        trough_inflation,
        steady_volatility,
        steady_dispersion,
        peak_max_dispersion,
        near_clamp,
        total_volume: world.market.total_volume(),
        inflation_trace,
        goods,
        outcomes,
    }
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 1.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

fn std_dev(xs: &[f64], mean: f64) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64;
    var.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_kind_persona_matches_the_built_policy() {
        use crate::policy::Persona;
        // The persona surfaced in gossip must match the strategy a PolicyKind
        // actually builds — pin the three-way mapping so a reorder of `make`
        // or a wrong `persona()` impl can't silently desync them.
        assert_eq!(PolicyKind::Greedy.make(0, 1).persona(), Persona::Greedy);
        assert_eq!(PolicyKind::Cautious.make(0, 1).persona(), Persona::Cautious);
        assert_eq!(PolicyKind::Random.make(0, 1).persona(), Persona::Reckless);
        assert_eq!(PolicyKind::Mixed.make(0, 1).persona(), Persona::Greedy);
        assert_eq!(PolicyKind::Mixed.make(1, 1).persona(), Persona::Cautious);
        assert_eq!(PolicyKind::Mixed.make(2, 1).persona(), Persona::Reckless);
    }

    #[test]
    fn a_small_sim_runs_and_trades() {
        let cfg = SimConfig {
            bots: 6,
            base_seed: 7,
            ticks: 1500,
            dt: 1.0,
            policy: PolicyKind::Mixed,
            ..SimConfig::default()
        };
        let report = run_sim(&cfg);
        // Bots should have traded enough to move the market off its baseline.
        assert!(report.total_volume > 0.0, "no trades happened");
        assert_eq!(report.inflation_trace.len(), cfg.ticks as usize);
        // Under sustained load the market should hold a sane steady state, not
        // run away to the clamps.
        assert!(
            report.steady_inflation > 0.3 && report.steady_inflation < 3.0,
            "steady inflation out of band: {}",
            report.steady_inflation
        );
        // Sustained mode respawns bots, so journeys keep completing.
        assert!(report.wins + report.losses >= 1, "no bot finished a journey");
    }

    #[test]
    fn report_serializes_to_json_and_csv() {
        let report = run_sim(&SimConfig {
            bots: 2,
            ticks: 300,
            ..SimConfig::default()
        });
        assert!(report.to_json().contains("final_inflation"));
        assert!(report.to_csv().starts_with("town,good,good_name"));
    }
}
