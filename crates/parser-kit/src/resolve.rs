//! Fuzzy single-token resolution against a candidate vocabulary.

use crate::config::LenThreshold;
use crate::distance::damerau_levenshtein_bounded;

/// The outcome of fuzzy-matching one token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match {
    /// Nothing within the edit-distance budget.
    None,
    /// Exactly one best candidate (or a tie broken by presence) — safe to apply
    /// silently.
    Unique(String),
    /// Several equally-close candidates, ranked present-first — needs the caller
    /// to confirm (or to disambiguate by context).
    Ambiguous(Vec<String>),
}

/// Find the closest vocabulary word(s) to `token`.
///
/// Candidates are scored by bounded Damerau–Levenshtein distance (budget from
/// `thresh(token.len())`); zero-distance hits are ignored (an exact match would
/// have resolved already, so the caller only fuzzes *unresolved* tokens). Among
/// the closest, a single winner is [`Match::Unique`]; a tie is broken toward a
/// candidate the `present` oracle accepts, and otherwise surfaced as
/// [`Match::Ambiguous`] ranked present-first then alphabetically.
pub fn best_match(
    token: &str,
    candidates: &[&str],
    thresh: LenThreshold,
    present: &dyn Fn(&str) -> bool,
) -> Match {
    let max = thresh(token.chars().count());
    if max == 0 {
        return Match::None;
    }

    let mut scored: Vec<(u32, &str)> = Vec::new();
    for &c in candidates {
        if let Some(d) = damerau_levenshtein_bounded(token, c, max) {
            if d > 0 {
                scored.push((d, c));
            }
        }
    }
    if scored.is_empty() {
        return Match::None;
    }

    let min_d = scored.iter().map(|(d, _)| *d).min().unwrap();
    let mut finalists: Vec<&str> = scored
        .iter()
        .filter(|(d, _)| *d == min_d)
        .map(|(_, c)| *c)
        .collect();
    finalists.sort_unstable();
    finalists.dedup();

    if finalists.len() == 1 {
        return Match::Unique(finalists[0].to_string());
    }

    // Tie: if exactly one finalist is present, context resolves it confidently.
    let present_finalists: Vec<&str> = finalists.iter().copied().filter(|c| present(c)).collect();
    if present_finalists.len() == 1 {
        return Match::Unique(present_finalists[0].to_string());
    }

    // Still ambiguous — rank present-first for the "did you mean?" suggestion.
    finalists.sort_by_key(|c| (!present(c), c.to_string()));
    Match::Ambiguous(finalists.into_iter().map(String::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(len: usize) -> u32 {
        match len {
            0..=3 => 0,
            4 => 1,
            _ => 2,
        }
    }

    const NONE: &dyn Fn(&str) -> bool = &|_| false;

    #[test]
    fn unique_correction() {
        let cands = ["lamp", "bird", "tablet"];
        assert_eq!(best_match("lmap", &cands, t, NONE), Match::Unique("lamp".into()));
    }

    #[test]
    fn short_tokens_are_not_corrected() {
        let cands = ["rod", "red", "rid"];
        assert_eq!(best_match("rad", &cands, t, NONE), Match::None);
    }

    #[test]
    fn no_candidate_within_budget() {
        let cands = ["lamp", "bird"];
        assert_eq!(best_match("qwerty", &cands, t, NONE), Match::None);
    }

    #[test]
    fn tie_is_ambiguous_without_presence() {
        let cands = ["cave", "case"];
        // "cale" is distance 1 from both (cave and case differ only at index 2).
        assert_eq!(
            best_match("cale", &cands, t, NONE),
            Match::Ambiguous(vec!["case".into(), "cave".into()])
        );
    }

    #[test]
    fn presence_breaks_a_tie() {
        let cands = ["cave", "case"];
        let present = |w: &str| w == "cave";
        assert_eq!(best_match("cale", &cands, t, &present), Match::Unique("cave".into()));
    }
}
