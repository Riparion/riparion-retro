//! Trader gossip: attributed bot activity the shared-economy server ships to
//! clients, so the human player's crew can talk about the *other* traders out on
//! the river — by name and by trading persona. Pure data; opt-in (only present in
//! multiplayer, never serialized into a save).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::market_link::Side;
use crate::policy::Persona;
use crate::scenario_data::scenario;

/// One thing a named trader did, for the crew to gossip about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GossipEvent {
    /// The trader's display name (e.g. "Lemuel Boggs").
    pub trader: String,
    /// Their trading temperament.
    pub persona: Persona,
    /// What they did.
    pub kind: GossipKind,
}

impl GossipEvent {
    /// Compose a crew remark — `(voice, line)` — from the scenario's gossip
    /// flavor, filling `{trader}`/`{persona}`/`{good}`/`{town}`/`{cause}`.
    /// Returns `None` when the scenario carries no gossip flavor or no phrasing
    /// for this kind. Deterministic (no RNG): voice/template vary by a hash of
    /// the trader name + kind, so repeated events read differently without state.
    pub fn compose(&self) -> Option<(String, String)> {
        let flavor = &scenario().gossip;
        if flavor.voices.is_empty() {
            return None;
        }
        let (kind_key, good, town) = self.kind.template_fields();
        // Prefer the tier-specific phrasing; fall back to the coarser key (e.g.
        // `trade_buy`) if the scenario didn't author the tiered variant.
        let phrasing = flavor
            .lines
            .iter()
            .find(|p| p.kind == kind_key && !p.templates.is_empty())
            .or_else(|| {
                self.kind
                    .fallback_key()
                    .and_then(|fk| flavor.lines.iter().find(|p| p.kind == fk && !p.templates.is_empty()))
            })?;
        let r = rotate(&self.trader, phrasing.kind.as_str());
        let voice = crate::flavor::pick(&flavor.voices, r).to_string();
        let template = crate::flavor::pick(&phrasing.templates, r);

        let persona = flavor
            .personas
            .iter()
            .find(|p| p.key == self.persona.key())
            .map(|p| p.epithet.as_str())
            .unwrap_or("");
        let good_name = good.map(crate::flavor::good_name).unwrap_or_default();
        let town_name = town.map(crate::flavor::town_name).unwrap_or_default();
        let cause = match &self.kind {
            GossipKind::Lost { cause } => cause.as_str(),
            _ => "",
        };

        let line = template
            .replace("{trader}", &self.trader)
            .replace("{persona}", persona)
            .replace("{good}", &good_name)
            .replace("{town}", &town_name)
            .replace("{cause}", cause);
        Some((voice, line))
    }
}

/// A stable, RNG-free rotation seed from the trader name + kind key (shared
/// FNV-1a). Returned as u64; callers reduce `% len` so the choice is identical
/// on wasm and native.
fn rotate(trader: &str, kind_key: &str) -> u64 {
    let mut buf = trader.as_bytes().to_vec();
    buf.extend_from_slice(kind_key.as_bytes());
    retro_core::hash::fnv1a(&buf)
}

/// Where a trade's price sat against the good's resting baseline at the time —
/// the actionable signal a connected player wants ("that town is bid up" /
/// "glutted, going cheap"). Computed by the world from the shared market; `Fair`
/// when no shared market is attached (the qty was notable but the price ordinary).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PriceTier {
    /// Mid well above baseline — prices bid up there.
    Premium,
    /// Mid near baseline — an ordinary price.
    Fair,
    /// Mid well below baseline — glutted, going cheap.
    Glut,
}

/// A noteworthy thing a trader did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GossipKind {
    /// A notable buy/sell at a landing, with where the price sat vs baseline.
    Trade {
        town: usize,
        good: usize,
        side: Side,
        qty: i64,
        tier: PriceTier,
    },
    /// Put in at a river landing (any town below Natchez).
    Arrived { town: usize },
    /// Reached Natchez — the end of the downstream run.
    ReachedNatchez,
    /// Sold the boat for lumber and started up the Natchez Trace.
    SetOutOnTrace,
    /// Robbed (river pirates or Trace bandits).
    Robbed,
    /// Made it home to Nashville.
    Won { score: i64 },
    /// Died or went broke; `cause` is the human-readable reason.
    Lost { cause: String },
}

impl GossipKind {
    /// The phrasing key for this event, plus the good/town indices to resolve
    /// `{good}`/`{town}` placeholders (when applicable).
    fn template_fields(&self) -> (&'static str, Option<usize>, Option<usize>) {
        match self {
            GossipKind::Trade { town, good, side, tier, .. } => (
                trade_key(*side, *tier),
                Some(*good),
                Some(*town),
            ),
            GossipKind::Arrived { town } => ("arrived", None, Some(*town)),
            GossipKind::ReachedNatchez => ("reached_natchez", None, None),
            GossipKind::SetOutOnTrace => ("set_out_on_trace", None, None),
            GossipKind::Robbed => ("robbed", None, None),
            GossipKind::Won { .. } => ("won", None, None),
            GossipKind::Lost { .. } => ("lost", None, None),
        }
    }

    /// A coarser phrasing key to fall back to when the tier-specific key has no
    /// templates authored — so a scenario can carry just `trade_buy`/`trade_sell`
    /// and still voice every trade.
    fn fallback_key(&self) -> Option<&'static str> {
        match self {
            GossipKind::Trade { side: Side::Buy, .. } => Some("trade_buy"),
            GossipKind::Trade { side: Side::Sell, .. } => Some("trade_sell"),
            _ => None,
        }
    }
}

/// The tier-specific phrasing key for a trade: side × price tier. A glutted buy
/// reads "loaded cheap", a premium sell reads "sold dear", and so on.
fn trade_key(side: Side, tier: PriceTier) -> &'static str {
    match (side, tier) {
        (Side::Buy, PriceTier::Premium) => "trade_buy_premium",
        (Side::Buy, PriceTier::Fair) => "trade_buy_fair",
        (Side::Buy, PriceTier::Glut) => "trade_buy_cheap",
        (Side::Sell, PriceTier::Premium) => "trade_sell_dear",
        (Side::Sell, PriceTier::Fair) => "trade_sell_fair",
        (Side::Sell, PriceTier::Glut) => "trade_sell_glut",
    }
}

/// A bounded ring of the most recent gossip, fed by the server and drained by
/// the engine's banter. Oldest-first; the banter takes from the front so the
/// player hears events roughly in the order they happened.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GossipFeed {
    events: VecDeque<GossipEvent>,
}

impl GossipFeed {
    /// Max events retained — stale gossip is dropped rather than queued forever.
    pub const CAP: usize = 16;

    /// Add an event, dropping the oldest if the ring is full.
    pub fn push(&mut self, event: GossipEvent) {
        if self.events.len() >= Self::CAP {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Take the oldest not-yet-spoken event, if any.
    pub fn take_oldest(&mut self) -> Option<GossipEvent> {
        self.events.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_fills_trader_persona_good_and_town() {
        // A notable buy → names the trader, the good, and the town.
        let e = GossipEvent {
            trader: "Lemuel Boggs".into(),
            persona: Persona::Greedy,
            kind: GossipKind::Trade {
                town: 5, // Louisville
                good: 1, // Whiskey
                side: Side::Buy,
                qty: 60,
                tier: PriceTier::Premium,
            },
        };
        let (voice, line) = e.compose().expect("kaintuck has gossip flavor");
        assert!(!voice.is_empty());
        assert!(line.contains("Lemuel Boggs"), "line: {line}");
        assert!(line.to_lowercase().contains("whiskey"), "line: {line}");
        // The town name resolved (not a leftover placeholder).
        assert!(!line.contains("{town}") && !line.contains("{good}"), "line: {line}");
    }

    #[test]
    fn trade_tier_changes_the_phrasing() {
        // Same trader/good/town/side, different price tier → different, signal-
        // bearing line (a premium buy reads "bid up", a glut buy reads "cheap").
        let mk = |tier| GossipEvent {
            trader: "Lemuel Boggs".into(),
            persona: Persona::Greedy,
            kind: GossipKind::Trade {
                town: 5,
                good: 1,
                side: Side::Buy,
                qty: 60,
                tier,
            },
        };
        let premium = mk(PriceTier::Premium).compose().unwrap().1;
        let glut = mk(PriceTier::Glut).compose().unwrap().1;
        assert_ne!(premium, glut, "tiers should read differently");
        assert!(premium.to_lowercase().contains("bid up"), "premium: {premium}");
        assert!(glut.to_lowercase().contains("cheap"), "glut: {glut}");
    }

    #[test]
    fn compose_resolves_cause_and_persona() {
        let e = GossipEvent {
            trader: "Big Reuben Pike".into(),
            persona: Persona::Reckless,
            kind: GossipKind::Lost {
                cause: "The boat broke up at the Falls.".into(),
            },
        };
        let (_voice, line) = e.compose().unwrap();
        assert!(line.contains("Big Reuben Pike"));
        assert!(line.contains("broke up"), "cause not interpolated: {line}");
        assert!(!line.contains('{'), "unresolved placeholder: {line}");
    }
}
