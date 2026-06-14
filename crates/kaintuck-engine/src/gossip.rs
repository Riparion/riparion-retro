//! Trader gossip: attributed bot activity the shared-economy server ships to
//! clients, so the human player's crew can talk about the *other* traders out on
//! the river — by name and by trading persona. Pure data; opt-in (only present in
//! multiplayer, never serialized into a save).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::market_link::Side;
use crate::policy::Persona;
use crate::scenario_data::scenario;
use crate::state::GOOD_NAMES;

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
        let phrasing = flavor.lines.iter().find(|p| p.kind == kind_key)?;
        if phrasing.templates.is_empty() {
            return None;
        }
        let r = rotate(&self.trader, kind_key);
        let voice = flavor.voices[(r % flavor.voices.len() as u64) as usize].clone();
        let template = &phrasing.templates[(r % phrasing.templates.len() as u64) as usize];

        let persona = flavor
            .personas
            .iter()
            .find(|p| p.key == self.persona.key())
            .map(|p| p.epithet.as_str())
            .unwrap_or("");
        let good_name = good
            .and_then(|g| GOOD_NAMES.get(g))
            .map(|g| g.to_lowercase())
            .unwrap_or_default();
        let town_name = town
            .and_then(|t| scenario().river.towns.get(t))
            .map(|t| t.name.as_str())
            .unwrap_or("");
        let cause = match &self.kind {
            GossipKind::Lost { cause } => cause.as_str(),
            _ => "",
        };

        let line = template
            .replace("{trader}", &self.trader)
            .replace("{persona}", persona)
            .replace("{good}", &good_name)
            .replace("{town}", town_name)
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

/// A noteworthy thing a trader did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GossipKind {
    /// A notable buy/sell at a landing.
    Trade {
        town: usize,
        good: usize,
        side: Side,
        qty: i64,
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
            GossipKind::Trade { town, good, side, .. } => (
                match side {
                    Side::Buy => "trade_buy",
                    Side::Sell => "trade_sell",
                },
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
