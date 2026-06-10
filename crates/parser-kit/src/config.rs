//! Caller-supplied tables that parameterize the inference pipeline. All content
//! here is game-specific; the [`engine`](crate::engine) consumes it generically.

use std::collections::{HashMap, HashSet};

/// Maps a (mis)typed token's character length to the maximum edit distance the
/// fuzzy matcher will tolerate. Returning `0` disables correction for that
/// length (sensible for very short tokens, where one edit reaches many words).
///
/// A typical implementation: `|len| match len { 0..=3 => 0, 4 => 1, _ => 2 }`.
pub type LenThreshold = fn(usize) -> u32;

/// The knobs and word tables the consumer hands to [`infer`](crate::engine::infer).
#[derive(Clone)]
pub struct InferConfig {
    /// Tokens dropped before re-parsing (e.g. `the`, `a`, `at`). Must not contain
    /// any real vocabulary word, or that word would be silently eaten.
    pub filler: HashSet<String>,
    /// Single-token fallbacks (`get` → `take`, `x` → `look`). Only applied to a
    /// token that does not already resolve, so an alias can never shadow vocab.
    pub aliases: HashMap<String, String>,
    /// Leading `(verb, particle)` bigrams collapsed to one canonical verb
    /// (`pick up` → `take`, `turn on` → `on`).
    pub phrasal: HashMap<(String, String), String>,
    /// The most words a single command may contain (e.g. `2` for verb+noun).
    pub max_words: usize,
    /// Length-scaled fuzzy-match tolerance.
    pub thresh: LenThreshold,
}
