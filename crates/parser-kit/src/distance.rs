//! Bounded Damerau–Levenshtein edit distance.
//!
//! Hand-rolled (no `strsim` dependency) to keep consumer binaries — notably WASM
//! game bundles — lean. We only ever ask "is the distance ≤ max?", so the routine
//! short-circuits as soon as a whole DP row exceeds `max`, and rejects up front
//! when the length difference alone already exceeds it.

/// Optimal-string-alignment Damerau–Levenshtein distance between `a` and `b`,
/// returning `None` as soon as it is known to exceed `max` (so callers pay only
/// for near matches). Counts a single adjacent transposition as one edit.
///
/// This is the OSA variant (each substring edited at most once); that is the
/// right model for typo correction and avoids the unbounded triangle-inequality
/// surprises of the full Damerau distance.
pub fn damerau_levenshtein_bounded(a: &str, b: &str, max: u32) -> Option<u32> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (la, lb) = (a.len(), b.len());

    // The length gap is a lower bound on the distance.
    let gap = (la as isize - lb as isize).unsigned_abs() as u32;
    if gap > max {
        return None;
    }

    // `prev2` = row i-2 (for transposition), `prev` = row i-1, `cur` = row i.
    let mut prev2: Vec<u32> = vec![0; lb + 1];
    let mut prev: Vec<u32> = (0..=lb as u32).collect();
    let mut cur: Vec<u32> = vec![0; lb + 1];

    for i in 1..=la {
        cur[0] = i as u32;
        let mut row_min = cur[0];
        for j in 1..=lb {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            let mut v = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            // Adjacent transposition (OSA).
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(prev2[j - 2] + 1);
            }
            cur[j] = v;
            row_min = row_min.min(v);
        }
        // Whole row already worse than the budget — no later row can recover.
        if row_min > max {
            return None;
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }

    let d = prev[lb];
    (d <= max).then_some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_is_zero() {
        assert_eq!(damerau_levenshtein_bounded("lamp", "lamp", 2), Some(0));
    }

    #[test]
    fn single_edits() {
        assert_eq!(damerau_levenshtein_bounded("lmap", "lamp", 2), Some(1)); // transpose
        assert_eq!(damerau_levenshtein_bounded("birdd", "bird", 2), Some(1)); // delete
        assert_eq!(damerau_levenshtein_bounded("doer", "door", 2), Some(1)); // substitute
        assert_eq!(damerau_levenshtein_bounded("tak", "take", 2), Some(1)); // insert
    }

    #[test]
    fn transpose_plus_insert() {
        // tabel -> table (transpose) -> tablet (insert): distance 2.
        assert_eq!(damerau_levenshtein_bounded("tabel", "tablet", 2), Some(2));
    }

    #[test]
    fn returns_none_past_budget() {
        assert_eq!(damerau_levenshtein_bounded("tabel", "tablet", 1), None);
        assert_eq!(damerau_levenshtein_bounded("qwerty", "lamp", 2), None);
    }

    #[test]
    fn length_gap_short_circuit() {
        // 5-char gap, budget 2 — rejected on the length check alone.
        assert_eq!(damerau_levenshtein_bounded("a", "abcdef", 2), None);
    }

    #[test]
    fn empty_strings() {
        assert_eq!(damerau_levenshtein_bounded("", "", 0), Some(0));
        assert_eq!(damerau_levenshtein_bounded("", "ab", 2), Some(2));
    }
}
