//! Presentation transform: the engine emits faithful ALL-CAPS teletype text (so
//! it matches the original byte-for-byte in tests); the UI lightly modernizes it
//! to mixed case for comfortable phone reading. This is the only place casing is
//! changed — parser echoes use the raw typed tokens, never this.

/// Convert ALL-CAPS engine output to sentence case: lowercase everything, then
/// capitalize the first letter of each sentence and the standalone word "I".
pub fn modernize(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut cap = true; // capitalize the next alphabetic char
    let mut prev_alnum = false;
    let mut chars = lower.chars().peekable();
    while let Some(c) = chars.next() {
        let next_alpha = chars.peek().map(|n| n.is_alphabetic()).unwrap_or(false);
        let standalone_i = c == 'i' && !prev_alnum && !next_alpha;
        if (cap || standalone_i) && c.is_alphabetic() {
            for u in c.to_uppercase() {
                out.push(u);
            }
            cap = false;
        } else {
            out.push(c);
        }
        // Only real sentence enders start a new capital. Newlines in the data
        // are line-wrapping inside a sentence, so they must NOT trigger caps
        // (else "...debris\nHere" would wrongly capitalize "Here").
        if matches!(c, '.' | '!' | '?') {
            cap = true;
        }
        prev_alnum = c.is_alphanumeric();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentence_case() {
        assert_eq!(
            modernize("YOU ARE STANDING AT THE END OF A ROAD. AROUND YOU IS A FOREST."),
            "You are standing at the end of a road. Around you is a forest."
        );
    }

    #[test]
    fn standalone_i() {
        assert_eq!(modernize("I WILL BE YOUR EYES"), "I will be your eyes");
        assert_eq!(modernize("WAIT, I'M STUCK"), "Wait, I'm stuck");
        // "in" must not become "In" mid-sentence.
        assert_eq!(modernize("YOU ARE IN A MAZE"), "You are in a maze");
    }

    #[test]
    fn newline_is_not_a_sentence_boundary() {
        // Line wrapping inside the data must not capitalize the next word.
        assert_eq!(
            modernize("PLUGGED WITH MUD AND DEBRIS\nHERE, BUT AN AWKWARD CANYON"),
            "Plugged with mud and debris\nhere, but an awkward canyon"
        );
    }
}
