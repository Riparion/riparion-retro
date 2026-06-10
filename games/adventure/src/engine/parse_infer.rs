//! Forgiving command interpretation for the limited Colossal Cave vocabulary.
//!
//! The exact-match parser in `game.rs` rejects anything it doesn't recognise
//! verbatim — `take the lamp`, `pick up lamp`, or a typo like `lmap` all fail.
//! This module feeds such near-misses through the game-agnostic [`parser_kit`]
//! engine, which strips filler, collapses phrasal verbs, expands aliases, drops
//! over-long input to a verb+noun pair, and fuzzy-corrects typos.
//!
//! Only the Cave-specific *content* lives here — the word tables and the fuzzy
//! candidate set. The mechanics, and the all-important "leave already-valid
//! input untouched" guarantee (driven by `Game::resolves_ok`), live in
//! `parser_kit`. See `game.rs::maybe_infer` for the wiring.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use parser_kit::InferConfig;

use super::data::data;

/// Full word spellings used as the fuzzy-match candidate set. Built once from
/// the parsed vocabulary (`display_words` is the de-duplicated, alphabetic
/// synonym list); the borrowed `&'static str`s point into the shared tables.
///
/// We match against full spellings rather than `text_to_n` keys so the 5-letter
/// truncations (`inven` alongside `inventory`) don't create phantom near-dupes;
/// an accepted word still re-resolves through `text_to_n`, which holds both.
pub fn candidates() -> &'static [&'static str] {
    static C: OnceLock<Vec<&'static str>> = OnceLock::new();
    C.get_or_init(|| data().display_words.iter().map(String::as_str).collect())
}

/// Length-scaled edit-distance budget. Short tokens get no correction (one edit
/// reaches too many three-letter words, e.g. `rod`/`red`); longer tokens allow
/// a transposition-plus-edit (e.g. `tabel`→`tablet`).
fn len_threshold(len: usize) -> u32 {
    match len {
        0..=3 => 0,
        4 => 1,
        _ => 2,
    }
}

/// The Cave's filler / alias / phrasal tables, assembled once.
pub fn infer_config() -> &'static InferConfig {
    static CFG: OnceLock<InferConfig> = OnceLock::new();
    CFG.get_or_init(|| {
        // Safe to drop: none of these is real Cave vocabulary. Deliberately
        // excludes real words that look like filler — up/down/in/out/on/off,
        // n/s/e/w/u/d, go/turn/back/look/it — so they're never eaten.
        let filler = ["the", "a", "an", "at", "to", "with", "my", "your", "some", "please", "of"]
            .into_iter()
            .map(String::from)
            .collect::<HashSet<_>>();

        // Modern phrasings the 1977 vocabulary lacks. Applied only to a token
        // that doesn't already resolve, so e.g. `get` (already a `carry`
        // synonym) is untouched and these never shadow real words.
        let aliases = [
            ("get", "take"),
            ("grab", "take"),
            ("pickup", "take"),
            ("x", "look"),
            ("l", "look"),
            ("i", "inventory"),
            ("inv", "inventory"),
            ("g", "go"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<HashMap<_, _>>();

        // Leading verb+particle bigrams collapsed to one canonical verb. `on`
        // and `off` are themselves real vocabulary (lamp on/off).
        let phrasal = [
            (("pick", "up"), "take"),
            (("put", "down"), "drop"),
            (("set", "down"), "drop"),
            (("turn", "on"), "on"),
            (("turn", "off"), "off"),
            (("look", "at"), "look"),
            (("look", "in"), "look"),
            (("throw", "at"), "throw"),
        ]
        .into_iter()
        .map(|((a, b), v)| ((a.to_string(), b.to_string()), v.to_string()))
        .collect::<HashMap<_, _>>();

        InferConfig {
            filler,
            aliases,
            phrasal,
            max_words: 2,
            thresh: len_threshold,
        }
    })
}
