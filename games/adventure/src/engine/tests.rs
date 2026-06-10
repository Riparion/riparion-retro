//! Engine tests. The headline tests replay python-adventure's seeded
//! walkthroughs (doctest golden masters) through our engine and compare the
//! output. Because we reproduce CPython's RNG bit-for-bit, every random event
//! (dwarves, pirate, pit deaths) lines up, so the whole playthrough must match.

use super::game::Game;

const WALK1: &str = include_str!("walkthroughs/walkthrough1.txt");
const WALK2: &str = include_str!("walkthroughs/walkthrough2.txt");
const WALK3: &str = include_str!("walkthroughs/walkthrough3.txt");
const WALK4: &str = include_str!("walkthroughs/walkthrough4.txt");

/// Collapse all runs of whitespace (including newlines) to single spaces and
/// trim. This compares *content* while ignoring the doctest console's 70-column
/// rewrapping versus our raw data newlines.
fn normalize(s: &str) -> String {
    s.replace("<BLANKLINE>", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Pull the lowercase command tokens out of a doctest `>>>` line such as
/// `get(water)` -> ["get", "water"] or `inven()` -> ["inven"].
fn tokens(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in content.chars() {
        if c.is_ascii_lowercase() || c == '?' {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn seed_of(content: &str) -> Option<u64> {
    let i = content.find("seed")?;
    let after = &content[i + 4..];
    let digits: String = after.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

enum Step {
    Play(u64),
    Cmd(String),
    /// `save(file)` — snapshot the game (and spend a turn, like python).
    SaveCheckpoint,
    /// `resume(file)` — restore the last snapshot.
    RestoreCheckpoint,
    /// A python REPL line (import/assignment/introspection) — ignore entirely.
    Skip,
}

fn classify(content: &str) -> Step {
    if content.contains("play") && seed_of(content).is_some() {
        Step::Play(seed_of(content).unwrap())
    } else if content.contains("resume") {
        Step::RestoreCheckpoint
    } else if content.starts_with("save(") {
        Step::SaveCheckpoint
    } else if content.starts_with("import")
        || content.contains('=')
        || content.contains('.')
        || content.contains('_')
        || content.contains("BytesIO")
    {
        Step::Skip
    } else {
        let toks = tokens(content);
        if toks.is_empty() {
            Step::Skip
        } else {
            Step::Cmd(toks.join(" "))
        }
    }
}

/// Replay one or more doctest sessions in `text`, asserting each step matches.
fn replay(name: &str, text: &str) {
    // Phase 1: parse the doctest into (step, expected-output) pairs. Output
    // lines attach to the most recent prompt.
    let mut steps: Vec<(Step, String)> = Vec::new();
    for raw in text.lines() {
        if let Some(content) = raw.strip_prefix(">>> ") {
            steps.push((classify(content), String::new()));
        } else if let Some((_, expected)) = steps.last_mut() {
            if !expected.is_empty() {
                expected.push('\n');
            }
            expected.push_str(raw);
        }
    }

    // Phase 2: execute, comparing each real command's output.
    let mut game: Option<Game> = None;
    let mut checkpoint: Option<Game> = None;
    let mut n = 0usize;
    for (step, expected) in &steps {
        match step {
            Step::Play(seed) => {
                let g = Game::started(*seed);
                let welcome = match g.transcript.last() {
                    Some(super::state::Line::Out(s)) => s.clone(),
                    _ => String::new(),
                };
                check(name, "play", n, expected, &welcome);
                game = Some(g);
                n = 0;
            }
            Step::Cmd(cmd) => {
                n += 1;
                if let Some(g) = game.as_mut() {
                    let out = g.command(cmd);
                    check(name, cmd, n, expected, &out);
                }
            }
            Step::SaveCheckpoint => {
                // python's `save(file)` spends a turn (no RNG) then pickles.
                if let Some(g) = game.as_mut() {
                    g.command("save x");
                    checkpoint = Some(g.clone());
                }
            }
            Step::RestoreCheckpoint => {
                if let Some(cp) = &checkpoint {
                    game = Some(cp.clone());
                }
            }
            Step::Skip => {}
        }
    }
}

fn check(name: &str, cmd: &str, step: usize, expected: &str, actual: &str) {
    let want = normalize(expected);
    let got = normalize(actual);
    assert!(
        want == got,
        "{name}: mismatch at step {step} after command {cmd:?}\n\
         --- expected ---\n{want}\n\
         --- actual ---\n{got}\n",
    );
}

/// Diagnostic: print the RNG op sequence per command for a walkthrough, to diff
/// against an instrumented python-adventure run. Run with `--ignored --nocapture`.
#[test]
#[ignore]
fn dump_rng_ops_walk2() {
    let mut game: Option<Game> = None;
    let mut n = 0;
    for raw in WALK2.lines() {
        if let Some(content) = raw.strip_prefix(">>> ") {
            match classify(content) {
                Step::Play(s) => {
                    let mut g = Game::started(s);
                    g.take_rng_ops();
                    game = Some(g);
                }
                Step::Cmd(cmd) => {
                    n += 1;
                    if let Some(g) = game.as_mut() {
                        g.command(&cmd);
                        let ops = g.take_rng_ops();
                        println!("{} {} {} {}", n, cmd, ops.len(), ops.join(""));
                    }
                }
                _ => {}
            }
        }
    }
}

#[test]
fn smoke_new_game_starts() {
    let mut g = Game::started(1);
    assert!(!g.transcript.is_empty());
    let out = g.command("no");
    assert!(out.contains("ROAD") || out.contains("BUILDING"), "got: {out}");
}

#[test]
fn walkthrough1_matches() {
    replay("walkthrough1", WALK1);
}

#[test]
fn walkthrough2_matches() {
    replay("walkthrough2", WALK2);
}

#[test]
fn walkthrough3_matches() {
    replay("walkthrough3", WALK3);
}

#[test]
fn walkthrough4_matches() {
    replay("walkthrough4", WALK4);
}

// --- forgiving input inference (parser-kit) ---------------------------------
//
// Each test compares a "messy" phrasing against the canonical command from a
// second identically-seeded game: if inference works, the engine produces the
// exact same turn (same output, same RNG draws). The four golden-master
// walkthroughs above prove the flip side — that already-valid input is untouched.

/// A game past the title prompt and stepped into the building, where the lamp,
/// food, keys, and bottle sit — so noun commands have objects to act on.
fn in_building(seed: u64) -> Game {
    let mut g = Game::started(seed);
    g.command("no"); // decline instructions
    g.command("enter"); // road -> inside building
    g
}

#[test]
fn filler_words_are_ignored() {
    let mut a = in_building(7);
    let mut b = in_building(7);
    assert_eq!(a.command("take lamp"), b.command("take the lamp"));
}

#[test]
fn phrasal_verbs_are_understood() {
    let mut a = in_building(7);
    let mut b = in_building(7);
    assert_eq!(a.command("take lamp"), b.command("pick up lamp"));
}

#[test]
fn aliases_map_onto_vocabulary() {
    let mut a = in_building(7);
    let mut b = in_building(7);
    assert_eq!(a.command("inventory"), b.command("i"));

    let mut a = in_building(7);
    let mut b = in_building(7);
    // `x` is not Cave vocabulary; `examine` is a native `look` synonym.
    assert_eq!(a.command("examine lamp"), b.command("x lamp"));
}

#[test]
fn extra_words_collapse_to_verb_noun() {
    let mut a = in_building(7);
    let mut b = in_building(7);
    assert_eq!(a.command("take lamp"), b.command("take the small brass lamp"));
}

#[test]
fn unambiguous_typos_are_corrected() {
    let mut a = in_building(7);
    let mut b = in_building(7);
    assert_eq!(a.command("take lamp"), b.command("take lmap"));
}

#[test]
fn gibberish_still_falls_through_to_dont_understand() {
    // No near match -> the original failure path runs unchanged; no false guess.
    let mut g = in_building(7);
    let out = g.command("qwerty zxcvbn").to_uppercase();
    assert!(!out.is_empty());
    assert!(!out.contains("DO YOU MEAN"), "should not invent a suggestion: {out}");
}

#[test]
fn ambiguous_typo_asks_for_confirmation() {
    let mut g = in_building(7);
    // "wace" is edit-distance 1 from both WAVE and WAKE.
    let out = g.command("wace");
    assert!(out.to_uppercase().contains("DO YOU MEAN"), "got: {out}");
    // Confirming runs the suggestion; a fresh non-yes/no reply would cancel.
    let yes = g.command("yes");
    assert!(!yes.is_empty(), "confirming should run the inferred command");
}

#[test]
fn declining_a_did_you_mean_bows_out() {
    let mut g = in_building(7);
    assert!(g.command("wace").to_uppercase().contains("DO YOU MEAN"));
    let no = g.command("no");
    assert!(!no.is_empty());
}

/// Mirrors python-adventure's test_commands: every vocabulary word, used both
/// alone and with an object, must run without panicking (no out-of-bounds, no
/// wedged state). A fresh game per word avoids cross-contamination.
#[test]
fn every_command_runs_without_panic() {
    let words: Vec<String> = super::data::data()
        .text_to_n
        .keys()
        .filter(|w| w.chars().all(|c| c.is_ascii_lowercase()))
        .cloned()
        .collect();
    for word in &words {
        let mut g = Game::started(7);
        g.command("no"); // decline instructions
        g.command(word); // intransitive
        let mut g = Game::started(7);
        g.command("no");
        g.command("enter"); // step next to the lamp
        g.command(&format!("{word} lamp")); // transitive
    }
}
