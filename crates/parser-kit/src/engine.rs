//! The tiered inference pipeline. See [`infer`].

use crate::config::InferConfig;
use crate::resolve::{best_match, Match};

/// What [`infer`] decided to do with a player's raw tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inference {
    /// The input already parses — the caller should proceed unchanged. (Returned
    /// for every command the caller's parser understands today, so the layer is
    /// inert on the happy path.)
    Unchanged,
    /// A normalized command (≤ `max_words`) that now resolves; run it.
    Rewrite(Vec<String>),
    /// A confident-but-ambiguous reconstruction the caller should confirm with
    /// the player ("did you mean …?").
    DidYouMean(Vec<String>),
    /// Nothing recoverable — the caller should run its own failure path.
    Unresolved,
}

/// Recover a runnable command from near-miss input.
///
/// `resolves_ok` answers "do these tokens parse as a command today?"; it gates
/// every tier, so currently-valid input returns [`Inference::Unchanged`] and is
/// never altered. `present` reports whether a candidate word names something in
/// scope, used to rank fuzzy matches. `candidates` is the full-spelling vocab.
///
/// Tiers, each tried only if the previous left the input unresolved: strip
/// filler words, then collapse a leading phrasal-verb bigram, then expand
/// single-token aliases (after each, also try shortening over-long input to a
/// resolving `max_words`-length subsequence), then fuzzy-correct unresolved
/// tokens, and finally offer a "did you mean?" from the best ambiguous candidate.
pub fn infer(
    raw: &[String],
    cfg: &InferConfig,
    candidates: &[&str],
    resolves_ok: &dyn Fn(&[String]) -> bool,
    present: &dyn Fn(&str) -> bool,
) -> Inference {
    // Tier 0 — the guarantee: already-valid input is left exactly as it is.
    if resolves_ok(raw) {
        return Inference::Unchanged;
    }

    let mut toks: Vec<String> = raw.to_vec();

    // Tier 1 — strip filler (but never strip away the whole command).
    let stripped: Vec<String> = toks
        .iter()
        .filter(|t| !cfg.filler.contains(*t))
        .cloned()
        .collect();
    if !stripped.is_empty() {
        toks = stripped;
    }
    if let Some(r) = settle(&toks, cfg.max_words, resolves_ok) {
        return Inference::Rewrite(r);
    }

    // Tier 2 — collapse a leading verb+particle bigram.
    toks = apply_phrasal(&toks, cfg);
    if let Some(r) = settle(&toks, cfg.max_words, resolves_ok) {
        return Inference::Rewrite(r);
    }

    // Tier 3 — expand single-token aliases (only where the token doesn't resolve).
    toks = apply_alias(&toks, cfg, resolves_ok);
    if let Some(r) = settle(&toks, cfg.max_words, resolves_ok) {
        return Inference::Rewrite(r);
    }

    // Tier 5 — fuzzy-correct the still-unresolved tokens in place.
    let mut ambiguous: Option<(usize, Vec<String>)> = None;
    let mut fuzzed = toks.clone();
    for (i, t) in toks.iter().enumerate() {
        if resolves_ok(std::slice::from_ref(t)) {
            continue; // already real vocabulary
        }
        match best_match(t, candidates, cfg.thresh, present) {
            Match::Unique(w) => fuzzed[i] = w,
            Match::Ambiguous(list) => {
                if ambiguous.is_none() {
                    ambiguous = Some((i, list));
                }
            }
            Match::None => {}
        }
    }
    toks = fuzzed;
    if let Some(r) = settle(&toks, cfg.max_words, resolves_ok) {
        return Inference::Rewrite(r);
    }

    // Tier 6 — offer the best ambiguous candidate as a "did you mean?".
    if let Some((i, list)) = ambiguous {
        if let Some(top) = list.first() {
            let mut guess = toks.clone();
            guess[i] = top.clone();
            if let Some(r) = settle(&guess, cfg.max_words, resolves_ok) {
                return Inference::DidYouMean(r);
            }
        }
    }

    Inference::Unresolved
}

/// Return a runnable command from `toks` if one is reachable without dropping a
/// short command's words: pass a ≤`max_words` command through unchanged when it
/// resolves, otherwise shorten an over-long one to the longest ordered
/// subsequence (preferring more words) that resolves.
fn settle(
    toks: &[String],
    max_words: usize,
    resolves_ok: &dyn Fn(&[String]) -> bool,
) -> Option<Vec<String>> {
    if toks.is_empty() {
        return None;
    }
    if toks.len() <= max_words {
        // Don't strip words out of an already-short command (that would turn
        // "take lmap" into "take" and lose the noun) — only fuzzy can help here.
        return resolves_ok(toks).then(|| toks.to_vec());
    }
    // Only shorten *toward* a multi-word command (never down to a bare single
    // word): if the player typed several words they meant a verb+noun, not just
    // the noun — collapsing "pick up lamp" to "lamp" would skip the phrasal tier.
    for len in (2..=max_words).rev() {
        for combo in combinations(toks.len(), len) {
            let words: Vec<String> = combo.iter().map(|&i| toks[i].clone()).collect();
            if resolves_ok(&words) {
                return Some(words);
            }
        }
    }
    None
}

/// All ordered index combinations `C(n, k)` (ascending indices).
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn rec(start: usize, n: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..n {
            cur.push(i);
            rec(i + 1, n, k, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    rec(0, n, k, &mut Vec::new(), &mut out);
    out
}

fn apply_phrasal(toks: &[String], cfg: &InferConfig) -> Vec<String> {
    if toks.len() >= 2 {
        let key = (toks[0].clone(), toks[1].clone());
        if let Some(verb) = cfg.phrasal.get(&key) {
            let mut out = vec![verb.clone()];
            out.extend_from_slice(&toks[2..]);
            return out;
        }
    }
    toks.to_vec()
}

fn apply_alias(
    toks: &[String],
    cfg: &InferConfig,
    resolves_ok: &dyn Fn(&[String]) -> bool,
) -> Vec<String> {
    toks.iter()
        .map(|t| {
            if !resolves_ok(std::slice::from_ref(t)) {
                if let Some(a) = cfg.aliases.get(t) {
                    return a.clone();
                }
            }
            t.clone()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    fn words(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    /// A tiny fake vocabulary standing in for a real game's parser. Verbs: take,
    /// drop, look, inventory, on. Nouns: lamp, bird, tablet, rod. Accepted shapes:
    /// a single known word, or verb+noun / noun+verb.
    struct Vocab;
    impl Vocab {
        const VERBS: &'static [&'static str] = &["take", "drop", "look", "inventory", "on", "throw"];
        const NOUNS: &'static [&'static str] = &["lamp", "bird", "tablet", "rod"];
        fn known(w: &str) -> bool {
            Self::VERBS.contains(&w) || Self::NOUNS.contains(&w)
        }
        fn resolves(ws: &[String]) -> bool {
            match ws.len() {
                1 => Self::known(&ws[0]),
                2 => {
                    let (a, b) = (ws[0].as_str(), ws[1].as_str());
                    (Self::VERBS.contains(&a) && Self::NOUNS.contains(&b))
                        || (Self::NOUNS.contains(&a) && Self::VERBS.contains(&b))
                }
                _ => false,
            }
        }
    }

    fn cfg() -> InferConfig {
        let filler = ["the", "a", "at", "to"].into_iter().map(String::from).collect::<HashSet<_>>();
        let aliases = [("get", "take"), ("x", "look"), ("i", "inventory")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        let phrasal = [(("pick", "up"), "take"), (("turn", "on"), "on"), (("look", "at"), "look")]
            .into_iter()
            .map(|((a, b), v)| ((a.to_string(), b.to_string()), v.to_string()))
            .collect::<HashMap<_, _>>();
        InferConfig {
            filler,
            aliases,
            phrasal,
            max_words: 2,
            thresh: |len| match len {
                0..=3 => 0,
                4 => 1,
                _ => 2,
            },
        }
    }

    fn run(input: &str, present: &dyn Fn(&str) -> bool) -> Inference {
        let cands = Vocab::NOUNS
            .iter()
            .chain(Vocab::VERBS.iter())
            .copied()
            .collect::<Vec<_>>();
        infer(&words(input), &cfg(), &cands, &|w| Vocab::resolves(w), present)
    }

    const NONE: &dyn Fn(&str) -> bool = &|_| false;
    const ALL: &dyn Fn(&str) -> bool = &|_| true;

    #[test]
    fn already_valid_is_unchanged() {
        assert_eq!(run("take lamp", NONE), Inference::Unchanged);
        assert_eq!(run("lamp", NONE), Inference::Unchanged);
        assert_eq!(run("inventory", NONE), Inference::Unchanged);
    }

    #[test]
    fn filler_is_stripped() {
        assert_eq!(run("take the lamp", NONE), Inference::Rewrite(words("take lamp")));
        assert_eq!(run("look at bird", NONE), Inference::Rewrite(words("look bird")));
    }

    #[test]
    fn lone_filler_stays_unresolved() {
        assert_eq!(run("the", NONE), Inference::Unresolved);
    }

    #[test]
    fn phrasal_collapses() {
        assert_eq!(run("pick up lamp", NONE), Inference::Rewrite(words("take lamp")));
        assert_eq!(run("turn on lamp", NONE), Inference::Rewrite(words("on lamp")));
    }

    #[test]
    fn alias_expands_only_unknown_tokens() {
        assert_eq!(run("x bird", NONE), Inference::Rewrite(words("look bird")));
        assert_eq!(run("i", NONE), Inference::Rewrite(words("inventory")));
    }

    #[test]
    fn collapses_long_input_to_pair() {
        assert_eq!(
            run("take the big brass lamp", NONE),
            Inference::Rewrite(words("take lamp"))
        );
    }

    #[test]
    fn fuzzy_corrects_unambiguous_typos() {
        assert_eq!(run("take lmap", NONE), Inference::Rewrite(words("take lamp")));
        assert_eq!(run("lmap", NONE), Inference::Rewrite(words("lamp")));
        assert_eq!(run("tablt", NONE), Inference::Rewrite(words("tablet")));
    }

    #[test]
    fn gibberish_is_unresolved() {
        assert_eq!(run("qwerty zxcvb", NONE), Inference::Unresolved);
    }

    // A vocab where "cave"/"case" are takeable nouns, used to force a fuzzy tie:
    // "cale" is edit-distance 1 from both.
    fn tie_resolves(ws: &[String]) -> bool {
        let known = |w: &str| ["take", "cave", "case"].contains(&w);
        match ws.len() {
            1 => known(&ws[0]),
            2 => ws[0] == "take" && ["cave", "case"].contains(&ws[1].as_str()),
            _ => false,
        }
    }

    #[test]
    fn ambiguous_typo_offers_did_you_mean() {
        let cands = ["cave", "case", "take"];
        let mut c = cfg();
        c.aliases.clear();
        let out = infer(&words("take cale"), &c, &cands, &tie_resolves, NONE);
        match out {
            // Tie ranked alphabetically when nothing is present -> top is "case".
            Inference::DidYouMean(w) => assert_eq!(w, words("take case")),
            other => panic!("expected DidYouMean, got {other:?}"),
        }
    }

    #[test]
    fn presence_makes_ambiguous_into_silent_rewrite() {
        let cands = ["cave", "case", "take"];
        let mut c = cfg();
        c.aliases.clear();
        let present = |w: &str| w == "cave";
        let out = infer(&words("take cale"), &c, &cands, &tie_resolves, &present);
        assert_eq!(out, Inference::Rewrite(words("take cave")));
    }

    #[test]
    fn does_not_drop_noun_from_short_command() {
        // "take lmap" must not settle to bare "take"; fuzzy fixes the noun.
        assert_ne!(run("take lmap", ALL), Inference::Rewrite(words("take")));
    }
}
