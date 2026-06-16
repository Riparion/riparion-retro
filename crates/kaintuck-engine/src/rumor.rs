//! Dockside rumors: advance word the crew picks up about how the *next* landing
//! will price a good — *"The harbormaster swears furs are dear down at
//! Louisville."* Each tip comes from a named source with a fixed reliability, so
//! some sources are worth trusting and some are wind. A tip is heard at one dock
//! (while the player can still load cargo) and proves out — or doesn't — at the
//! next, turning crew banter into a real, imperfect-information trading edge.
//!
//! Single-player and fully deterministic: the next town's prices are pre-rolled
//! the moment you arrive at the current one (so a tip's truth is fixed when it's
//! spoken), and the [`Rumor`] lives in `GameState`. Generation consumes the main
//! RNG stream (source, the reliability gate, the lie); phrasing selection is
//! RNG-free (FNV-1a), so the golden trace stays reproducible.

use serde::{Deserialize, Serialize};

use retro_core::rng::GameRng;

use crate::prices;
use crate::scenario_data::scenario;
use crate::state::NUM_GOODS;

/// Advance word about how `town` will price `good`, from `source` (a source key).
/// `claimed_band` is what the tip *says* (1 = cheap/glut, 2 = ordinary,
/// 3 = dear/scarce). Whether the tip *holds* is not stored — it's the single
/// derived fact `claimed_band == band_of(revealed prices)`, recomputed at the
/// next dock against the prices the player actually faces (see [`Rumor::held`]),
/// so it can never drift from what was committed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rumor {
    pub source: String,
    pub town: usize,
    pub good: usize,
    pub claimed_band: u8,
}

/// The 1/2/3 price band of `price` for `good` at `town`, against the RNG-free
/// rank-mean [`prices::expected_mid`]. Prices are `rank_mean * band` with the
/// band an integer 1..=3, so this recovers it; clamped for safety. This is the
/// *offline* classifier of the per-visit price roll; the live shared-market
/// signal on trader gossip is a different bucketer (`world::price_tier`, which
/// grades a drifting mid against its relaxing baseline) — deliberately separate.
pub fn band_of(price: f64, town: usize, good: usize) -> u8 {
    let mean = prices::expected_mid(town, good);
    if mean <= 0.0 {
        return 2;
    }
    (price / mean).round().clamp(1.0, 3.0) as u8
}

/// The phrasing key for a band.
fn band_key(band: u8) -> &'static str {
    match band {
        1 => "cheap",
        3 => "dear",
        _ => "ordinary",
    }
}

/// Roll a rumor about `town`'s pre-committed `prices`, or `None` if the scenario
/// authored no rumor sources. The tip is about the good with the most extreme
/// band (the one most worth talking about — ties to the lowest index, RNG-free);
/// the source, the reliability gate, and (when the source lies) the wrong band
/// are drawn from the main stream, in that fixed order.
pub fn generate(rng: &mut GameRng, town: usize, prices: &[f64; NUM_GOODS]) -> Option<Rumor> {
    let flavor = &scenario().rumors;
    if flavor.sources.is_empty() {
        return None;
    }
    // The good worth a word: most extreme band, ties to the LOWEST index. No RNG
    // (depends only on the committed prices), so the subject is stable per run.
    // `.rev()` makes `max_by_key` (which keeps the last maximum it sees) land on
    // the smallest index among equally-extreme goods.
    let good = (0..NUM_GOODS)
        .rev()
        .max_by_key(|&g| (band_of(prices[g], town, g) as i32 - 2).abs())
        .unwrap_or(0);
    let actual = band_of(prices[good], town, good);

    let source = &flavor.sources[rng.ri(flavor.sources.len() as i64) as usize];
    let key = source.key.clone();
    let reliability = source.reliability;

    let true_tip = rng.uniform() < reliability;
    let claimed_band = if true_tip {
        actual
    } else {
        // Lie: pick one of the two other bands (bands are 1/2/3), in ascending
        // order so the RNG index maps the same way the old filter did.
        let others = match actual {
            1 => [2u8, 3],
            3 => [1u8, 2],
            _ => [1u8, 3],
        };
        others[rng.ri(others.len() as i64) as usize]
    };

    Some(Rumor { source: key, town, good, claimed_band })
}

/// A stable, RNG-free rotation seed from the source + target (shared FNV-1a), so
/// a given tip reads the same line every time without storing the choice.
fn rotate(source: &str, town: usize, good: usize) -> u64 {
    let mut buf = source.as_bytes().to_vec();
    buf.push(town as u8);
    buf.push(good as u8);
    retro_core::hash::fnv1a(&buf)
}

fn fill(template: &str, good: usize, town: usize) -> String {
    template
        .replace("{good}", &crate::flavor::good_name(good))
        .replace("{town}", &crate::flavor::town_name(town))
}

impl Rumor {
    /// Whether the tip holds against the prices the player actually faces at the
    /// landing — the single derived truth (`claimed_band == band_of(prices)`).
    /// Recomputed from the revealed prices rather than stored, so it can't drift
    /// from what was committed.
    pub fn held(&self, prices: &[f64; NUM_GOODS]) -> bool {
        self.claimed_band == band_of(prices[self.good], self.town, self.good)
    }

    /// A stable notification key for this tip, so the payoff heard at the next
    /// landing can be appended onto the tip's report rather than logged anew. The
    /// river is one-directional, so a (town, good) pair names a single tip per run.
    pub fn report_key(&self) -> String {
        format!("rumor:{}:{}", self.town, self.good)
    }

    /// Compose the tip as `(voice, line)` from the scenario flavor, filling
    /// `{good}`/`{town}`; the voice names the teller. `None` if the source or the
    /// claimed band carries no phrasing. RNG-free.
    pub fn compose(&self) -> Option<(String, String)> {
        let flavor = &scenario().rumors;
        let source = flavor.sources.iter().find(|s| s.key == self.source)?;
        let kind = band_key(self.claimed_band);
        let phrasing = flavor
            .lines
            .iter()
            .find(|p| p.kind == kind && !p.templates.is_empty())?;
        let r = rotate(&self.source, self.town, self.good);
        let template = crate::flavor::pick(&phrasing.templates, r);
        Some((source.voice.clone(), fill(template, self.good, self.town)))
    }

    /// Compose the payoff line shown on arrival — whether the tip `held` (true) or
    /// was wind (false) — keyed `"held"`/`"wind"` in `confirms`. `None` if
    /// unauthored. The caller passes the verdict from [`Rumor::held`] against the
    /// revealed prices.
    pub fn resolve_line(&self, held: bool) -> Option<String> {
        let flavor = &scenario().rumors;
        let kind = if held { "held" } else { "wind" };
        let phrasing = flavor
            .confirms
            .iter()
            .find(|p| p.kind == kind && !p.templates.is_empty())?;
        // Offset the rotation from the tip's so the payoff line isn't lockstep
        // with the tip when both pools are the same size.
        let r = rotate(&self.source, self.town, self.good).wrapping_add(0x9e37_79b9);
        let template = crate::flavor::pick(&phrasing.templates, r);
        Some(fill(template, self.good, self.town))
    }
}
