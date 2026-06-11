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

/// Width at or above which a line is treated as 80-column wrap residue rather
/// than a deliberate break. The original data wraps prose at ~70 columns, so a
/// near-full line almost certainly continues on the next; short lines (inventory
/// items, headings, centered dividers) ended on purpose. 50 leaves margin for
/// lines that wrapped a little early.
const WRAP_THRESHOLD: usize = 50;

/// Split one engine output block into display paragraphs, de-wrapping the
/// teletype line breaks baked into the data while keeping intentional structure.
///
/// Blank lines separate paragraphs. Within a paragraph, a line is joined to the
/// next with a single space only when it looks like wrap residue: no leading
/// whitespace, at least [`WRAP_THRESHOLD`] chars, and a non-blank, non-indented
/// follower. Otherwise the break survives as a `\n` — so inventory items stay
/// one-per-line and centered dividers keep their leading spaces.
pub fn reflow(s: &str) -> Vec<String> {
    let lines: Vec<&str> = s.split('\n').collect();
    let mut paragraphs: Vec<String> = Vec::new();
    let mut cur = String::new();

    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            if !cur.is_empty() {
                paragraphs.push(std::mem::take(&mut cur));
            }
            continue;
        }
        cur.push_str(line.trim_end());
        // Choose the separator to the next non-blank line, if any.
        match lines.get(i + 1) {
            Some(next) if !next.trim().is_empty() => {
                let wrap = !line.starts_with(char::is_whitespace)
                    && !next.starts_with(char::is_whitespace)
                    && line.trim_end().chars().count() >= WRAP_THRESHOLD;
                cur.push(if wrap { ' ' } else { '\n' });
            }
            // Blank follower flushes the paragraph on the next iteration; end of
            // input flushes after the loop.
            _ => {}
        }
    }
    if !cur.is_empty() {
        paragraphs.push(cur);
    }
    paragraphs
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

    #[test]
    fn reflow_dewraps_a_room_description() {
        // The three 80-column source lines collapse into one flowing paragraph
        // with no internal newline; the original double space after "." stays.
        let room = "YOU ARE STANDING AT THE END OF A ROAD BEFORE A SMALL BRICK BUILDING.\n\
                    AROUND YOU IS A FOREST.  A SMALL STREAM FLOWS OUT OF THE BUILDING AND\n\
                    DOWN A GULLY.";
        let paras = reflow(room);
        assert_eq!(paras.len(), 1);
        assert_eq!(
            paras[0],
            "YOU ARE STANDING AT THE END OF A ROAD BEFORE A SMALL BRICK BUILDING. \
             AROUND YOU IS A FOREST.  A SMALL STREAM FLOWS OUT OF THE BUILDING AND DOWN A GULLY."
        );
        assert!(!paras[0].contains('\n'));
    }

    #[test]
    fn reflow_keeps_inventory_one_per_line() {
        // Header, blank, then short item lines: two paragraphs, items keep breaks.
        let inv = "YOU ARE CURRENTLY HOLDING THE FOLLOWING:\n\nSET OF KEYS\nBRASS LANTERN\nBLACK ROD";
        let paras = reflow(inv);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0], "YOU ARE CURRENTLY HOLDING THE FOLLOWING:");
        assert_eq!(paras[1], "SET OF KEYS\nBRASS LANTERN\nBLACK ROD");
    }

    #[test]
    fn reflow_preserves_centered_divider() {
        // Leading whitespace marks a deliberately positioned line; keep it intact.
        let div = "                              - - -";
        assert_eq!(reflow(div), vec![div.to_string()]);
    }

    #[test]
    fn reflow_splits_paragraphs_on_blank_lines() {
        assert_eq!(
            reflow("OK.\n\nYOU ARE IN A MAZE."),
            vec!["OK.".to_string(), "YOU ARE IN A MAZE.".to_string()]
        );
    }
}
