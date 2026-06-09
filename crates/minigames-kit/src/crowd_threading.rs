//! CrowdThreading — a route-memory test through a crowd.
//!
//! The player gets a brief overhead snapshot of a crowd or corridor: their
//! position, the exit, a scatter of obstacles (people, furniture, guards), and a
//! lit path threading from start to exit. Then the map "closes" and they must
//! reproduce the route from memory by tapping a sequence of direction choices.
//! It rewards spatial memory, not reflexes — a wrong turn means you ran into
//! someone, not that your fingers were slow. The component reports back how far
//! they got via [`CrowdThreadingResult`]; the host game turns that into a score
//! (a clean getaway, a scuffle, a chase).
//!
//! It scales from a single knob: `crowd_size` drives the grid size, the route
//! length, and the density, so a thin crowd is a short sequence and a packed
//! room is a long one with more chances to misremember. The crowd-member icon
//! (and the player/exit markers) are parameterized too.
//!
//! The level — grid, route, obstacle placement — is a pure function of a
//! caller-supplied `seed`, so runs are deterministic and replayable; a fresh
//! seed per encounter keeps the route from being memorized across runs.
//!
//! Timing uses a 20 ms tick counter (gloo-timers) rather than a wall clock, so
//! the reveal countdown is a pure function of elapsed ticks and the component
//! adds no nondeterminism of its own.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use retro_kit::rng::GameRng;
use retro_kit::theme::{BTN, SCREEN_CENTERED};

/// Granularity of the reveal countdown, in milliseconds (~50 fps).
pub const TICK_MS: u32 = 20;

/// The outcome of one CrowdThreading round, handed to the host's `on_complete`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrowdThreadingResult {
    /// Whether the whole route was reproduced correctly (reached the exit).
    pub cleared: bool,
    /// Correct turns made before finishing or colliding.
    pub steps_taken: usize,
    /// Total turns in the route.
    pub path_len: usize,
    /// `steps_taken / path_len`, 0.0..1.0 (1.0 on a clean clear).
    pub accuracy: f64,
}

/// A step on the grid. `Up` is toward the exit (decreasing row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    /// The arrow glyph shown on the direction button.
    pub fn glyph(self) -> &'static str {
        match self {
            Dir::Up => "↑",
            Dir::Down => "↓",
            Dir::Left => "←",
            Dir::Right => "→",
        }
    }

    /// Column/row delta. Rows increase downward, so `Up` is `(0, -1)`.
    pub fn step(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }
}

/// The four directions in button order.
pub const DIRS: [Dir; 4] = [Dir::Up, Dir::Left, Dir::Right, Dir::Down];

/// A generated level: the grid, the route through it, and the crowd around it.
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub cols: usize,
    pub rows: usize,
    /// Player start (`col`, `row`), on the bottom row.
    pub start: (usize, usize),
    /// Exit (`col`, `row`), on the top row.
    pub exit: (usize, usize),
    /// The route as a turn sequence — what the player must reproduce.
    pub path: Vec<Dir>,
    /// Cells the route passes through, start→exit (drawn as the lit trail).
    pub walked: Vec<(usize, usize)>,
    /// Crowd-member cells (never on the route).
    pub obstacles: Vec<(usize, usize)>,
}

/// Derive a square-ish grid that comfortably holds the crowd plus a route.
/// Grows with `crowd_size`, clamped so it always fits a phone screen.
pub fn grid_dims(crowd_size: usize) -> (usize, usize) {
    // Want roughly twice as many cells as crowd members, plus room for the
    // route; take the side of that square and clamp to a sane on-screen range.
    let side = ((crowd_size * 2 + 4) as f64).sqrt().ceil() as usize + 1;
    let side = side.clamp(4, 8);
    (side, side)
}

/// Build a level from `crowd_size` and `seed`. Deterministic: same inputs →
/// same grid, route, and crowd; vary `seed` per encounter for fresh layouts.
pub fn build_level(crowd_size: usize, seed: u64) -> Level {
    let (cols, rows) = grid_dims(crowd_size);
    let mut rng = GameRng::from_seed(seed);

    // Random column on the bottom row, so the route's foot moves with the seed
    // too (the upward carve still always reaches the top row).
    let start = (rng.ri(cols as i64) as usize, rows - 1);
    let (path, walked) = carve_route(cols, rows, start, &mut rng);
    let exit = *walked.last().unwrap_or(&start);

    // Crowd fills non-route cells, capped at how many are free.
    let mut free: Vec<(usize, usize)> = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            if !walked.contains(&(c, r)) {
                free.push((c, r));
            }
        }
    }
    // Deterministic Fisher–Yates, then take the first `crowd_size`.
    for i in (1..free.len()).rev() {
        let j = rng.ri(i as i64 + 1) as usize;
        free.swap(i, j);
    }
    let take = crowd_size.min(free.len());
    let mut obstacles: Vec<(usize, usize)> = free.into_iter().take(take).collect();
    obstacles.sort();

    Level {
        cols,
        rows,
        start,
        exit,
        path,
        walked,
        obstacles,
    }
}

/// Carve a self-avoiding route from `start` up to the top row, biased upward
/// with the occasional sidestep. Backtracks out of dead-ends, so it always
/// reaches row 0. Returns the turn sequence and the cells it passes through.
fn carve_route(
    cols: usize,
    rows: usize,
    start: (usize, usize),
    rng: &mut GameRng,
) -> (Vec<Dir>, Vec<(usize, usize)>) {
    let mut walked: Vec<(usize, usize)> = vec![start];
    let mut path: Vec<Dir> = Vec::new();
    // The walk reaches the exit the moment it steps onto the top row.
    while walked.last().unwrap().1 > 0 {
        let &(c, r) = walked.last().unwrap();
        // Candidate moves that stay in bounds and don't revisit a cell. Bias
        // upward by listing `Up` twice when it's available.
        let mut cands: Vec<Dir> = Vec::new();
        let consider = |dir: Dir, cands: &mut Vec<Dir>| {
            let (dc, dr) = dir.step();
            let nc = c as i32 + dc;
            let nr = r as i32 + dr;
            if nc >= 0 && nc < cols as i32 && nr >= 0 && nr < rows as i32 {
                let cell = (nc as usize, nr as usize);
                if !walked.contains(&cell) {
                    cands.push(dir);
                }
            }
        };
        consider(Dir::Up, &mut cands);
        if !cands.is_empty() {
            cands.push(Dir::Up); // weight progress toward the exit
        }
        consider(Dir::Left, &mut cands);
        consider(Dir::Right, &mut cands);

        if cands.is_empty() {
            // Dead-end: backtrack one cell and try again.
            if path.pop().is_some() {
                walked.pop();
                continue;
            }
            break; // should not happen from a bottom-row start
        }
        let dir = cands[rng.ri(cands.len() as i64) as usize];
        let (dc, dr) = dir.step();
        let cell = ((c as i32 + dc) as usize, (r as i32 + dr) as usize);
        walked.push(cell);
        path.push(dir);
    }
    (path, walked)
}

/// A route-memory minigame. Mount it (with a `key` that changes per encounter,
/// so the level and progress reset) and wire `on_complete`:
///
/// ```rust,ignore
/// CrowdThreading {
///     key: "{round}",
///     prompt: "Thread through the crowd —",
///     crowd_size: 10,    // thin crowd → short route; packed → long
///     member_icon: "🧍",
///     seed,              // e.g. drawn from the game's RNG
///     on_complete: move |r: CrowdThreadingResult| { /* score it */ },
/// }
/// ```
#[component]
pub fn CrowdThreading(
    /// Line shown above the grid (the situation: "Thread through the crowd —").
    prompt: String,
    /// Number of crowd members. Scales the grid, the route length, and the
    /// density — the one knob that turns this from easy to hard.
    crowd_size: usize,
    /// Glyph drawn for each crowd member. Defaults to a phosphor-friendly
    /// roguelike `&`; pass an emoji (e.g. "🧍") on hosts with an emoji font.
    #[props(default = String::from("&"))]
    member_icon: String,
    /// Glyph drawn at the player's start cell. Defaults to `@`.
    #[props(default = String::from("@"))]
    player_icon: String,
    /// Glyph drawn at the exit cell. Defaults to `>` (the way out).
    #[props(default = String::from(">"))]
    exit_icon: String,
    /// How long the snapshot stays up before the map closes, in milliseconds.
    #[props(default = 2600)]
    reveal_ms: u32,
    /// How long the player has to walk the route once the map closes, in
    /// milliseconds. If the timer runs out before they reach the exit, the run
    /// fails (counts as a collision: `cleared: false`, with partial progress).
    #[props(default = 10_000)]
    navigate_ms: u32,
    /// Seed for the level layout. Same seed → same level (deterministic); vary
    /// it per encounter so the route can't be memorized across runs.
    #[props(default = 0)]
    seed: u64,
    /// Called once, when the player clears the route or runs into someone.
    on_complete: EventHandler<CrowdThreadingResult>,
) -> Element {
    // Pinned once per mount.
    let level = use_signal(move || build_level(crowd_size, seed));
    let mut revealed = use_signal(|| true);
    let mut ticks = use_signal(|| 0u32);
    let mut nav_ticks = use_signal(|| 0u32);
    let mut step = use_signal(|| 0usize);
    let mut done = use_signal(|| false);

    // Count down the reveal; when it elapses, close the map.
    use_future(move || async move {
        loop {
            if !revealed() {
                break;
            }
            TimeoutFuture::new(TICK_MS).await;
            ticks += 1;
            if ticks() * TICK_MS >= reveal_ms {
                revealed.set(false);
                break;
            }
        }
    });

    // Phase-2 timer: once the map closes the player has `navigate_ms` to reach
    // the exit. If it elapses first the run fails, scored like a collision at
    // whatever step they'd reached.
    use_future(move || async move {
        loop {
            if done() {
                break;
            }
            // Idle until the navigate phase begins (GO pressed or reveal ends).
            if revealed() {
                TimeoutFuture::new(TICK_MS).await;
                continue;
            }
            TimeoutFuture::new(TICK_MS).await;
            nav_ticks += 1;
            if nav_ticks() * TICK_MS >= navigate_ms {
                if !done() {
                    done.set(true);
                    let taken = step();
                    let n = level.read().path.len();
                    on_complete.call(CrowdThreadingResult {
                        cleared: false,
                        steps_taken: taken,
                        path_len: n,
                        accuracy: if n == 0 { 0.0 } else { taken as f64 / n as f64 },
                    });
                }
                break;
            }
        }
    });

    let lvl = level.read();
    let path_len = lvl.path.len();
    // Where the player marker sits right now: the start during the reveal, then
    // one cell further up the route for each correct turn. On a wrong turn `step`
    // freezes, so the marker stops at the last cell they reached.
    let player_cell = lvl.walked.get(step()).copied().unwrap_or(lvl.start);

    // Reveal countdown as a fraction remaining (1.0 → 0.0).
    let remaining = if revealed() {
        let elapsed = (ticks() * TICK_MS).min(reveal_ms) as f64;
        (1.0 - elapsed / reveal_ms.max(1) as f64).max(0.0)
    } else {
        0.0
    };

    // Phase-2 timer as whole seconds remaining, shown in the D-pad center. The
    // last few seconds turn urgent so the deadline reads at a glance.
    let nav_left_ms = (navigate_ms as i64 - (nav_ticks() * TICK_MS) as i64).max(0);
    let nav_secs = ((nav_left_ms as f64) / 1000.0).ceil() as u32;
    let nav_urgent = nav_left_ms <= 3000;

    // Resolve a tapped direction during the navigate phase.
    let mut tap = move |dir: Dir| {
        if done() || revealed() {
            return;
        }
        let lvl = level.read();
        let correct = lvl.path.get(step()).copied();
        if correct == Some(dir) {
            step += 1;
            if step() >= lvl.path.len() {
                done.set(true);
                let n = lvl.path.len();
                on_complete.call(CrowdThreadingResult {
                    cleared: true,
                    steps_taken: n,
                    path_len: n,
                    accuracy: 1.0,
                });
            }
        } else {
            // Wrong turn — ran into someone.
            done.set(true);
            let taken = step();
            let n = lvl.path.len();
            on_complete.call(CrowdThreadingResult {
                cleared: false,
                steps_taken: taken,
                path_len: n,
                accuracy: if n == 0 { 0.0 } else { taken as f64 / n as f64 },
            });
        }
    };

    rsx! {
        // The whole layout is set inline, not just via the Tailwind classes: the
        // grid is styled inline by design, and some hosts (e.g. the Tailwind Play
        // CDN) don't emit the flex utilities for this dynamically-rendered DOM, so
        // `flex flex-col items-center justify-center` silently fall back to block
        // flow and nothing centers. The classes stay for hosts that do build them.
        div {
            class: "{SCREEN_CENTERED} gap-2 items-center",
            style: "display: flex; flex-direction: column; align-items: center; \
                justify-content: center; flex: 1; gap: 0.5rem; padding: 1rem; \
                width: 100%; max-width: 28rem; margin: 0 auto;",
            p { class: "text-lg opacity-80", style: "text-align: center;", "{prompt}" }

            // The overhead grid. During the reveal it shows the lit route and the
            // crowd; once the map closes it dims to a blank lattice — you're on
            // your own. Styled inline so the core visual needs no utilities.
            div {
                style: "display: grid; width: 100%; max-width: 19rem; aspect-ratio: 1; \
                    grid-template-columns: repeat({lvl.cols}, 1fr); gap: 2px; \
                    padding: 4px; border: 1px solid var(--phosphor-dim); border-radius: 4px; \
                    background: rgba(20, 95, 40, 0.10);",
                for r in 0..lvl.rows {
                    for c in 0..lvl.cols {
                        {
                            let cell = (c, r);
                            let is_player = cell == player_cell;
                            let is_exit = cell == lvl.exit;
                            // When the map closes, the lit route hides, leaving the
                            // player to reproduce the path from memory — but the crowd
                            // stays up (see below) as the obstacles to navigate around.
                            let on_trail = revealed() && lvl.walked.contains(&cell);
                            // Crowd icons stay up in both phases: the lit route hides
                            // when the map closes, but the people remain so the player
                            // navigates around them from memory.
                            let is_obstacle = lvl.obstacles.contains(&cell);
                            // Phase-2 cells the player has already walked — the start
                            // through the current position — get a fill and a lit border
                            // to trace the route taken so far.
                            let traversed = !revealed()
                                && lvl.walked.iter().take(step() + 1).any(|w| *w == cell);
                            // The lit route must read clearly against empty cells:
                            // a bright fill with an inset glow, vs. a near-black lane.
                            let cell_bg = if on_trail {
                                "background: rgba(51, 255, 102, 0.55); \
                                 box-shadow: inset 0 0 8px rgba(51, 255, 102, 0.7);"
                            } else if traversed {
                                // Phase-2 cells the player has walked: a soft fill so
                                // the route taken so far reads as a filled lane. Clear
                                // the box-shadow explicitly so the phase-1 trail glow
                                // doesn't linger (Dioxus won't drop an omitted prop).
                                "background: rgba(51, 255, 102, 0.30); box-shadow: none;"
                            } else {
                                // box-shadow set explicitly so the trail glow is
                                // cleared when the map closes (Dioxus patches style
                                // props individually and won't drop an omitted one).
                                "background: rgba(51, 255, 102, 0.04); box-shadow: none;"
                            };
                            // Player wins the cell when it lands on the exit, so you
                            // see the marker arrive rather than the exit swallowing it.
                            let glyph = if is_player {
                                player_icon.clone()
                            } else if is_exit {
                                exit_icon.clone()
                            } else if is_obstacle {
                                member_icon.clone()
                            } else {
                                String::new()
                            };
                            // Styling for the glyph only — kept off the cell div so the
                            // lattice (border + background) stays identical everywhere
                            // and never marks which cells are blocked. Player and exit
                            // pop; the crowd (shown only during the reveal) is dimmer so
                            // the lit route reads against it.
                            let glyph_style = if is_exit || is_player {
                                "color: var(--phosphor); text-shadow: var(--glow);"
                            } else {
                                "color: var(--phosphor-dim);"
                            };
                            // The per-cell border draws the lattice during the reveal;
                            // once the map closes (phase 2) it's cleared so the grid
                            // reads as open space — traversed cells included, which are
                            // marked by their fill alone. Set explicitly rather than
                            // omitted — Dioxus patches style props individually and
                            // won't drop one.
                            let cell_border = if revealed() {
                                "border: 1px solid var(--phosphor-dim);"
                            } else {
                                "border: 1px solid transparent;"
                            };
                            rsx! {
                                div {
                                    key: "{r}-{c}",
                                    style: "display: flex; align-items: center; justify-content: center; \
                                        aspect-ratio: 1; font-size: 1.35rem; font-weight: 700; \
                                        border-radius: 3px; {cell_bg} {cell_border}",
                                    span { style: "{glyph_style}", "{glyph}" }
                                }
                            }
                        }
                    }
                }
            }

            if revealed() {
                // Countdown bar plus the chance to commit early.
                div {
                    style: "position: relative; width: 100%; max-width: 19rem; height: 0.5rem; \
                        border: 1px solid var(--phosphor-dim); border-radius: 3px; overflow: hidden;",
                    div {
                        style: "position: absolute; top: 0; bottom: 0; left: 0; \
                            width: {remaining * 100.0}%; background: var(--phosphor); \
                            box-shadow: var(--glow);",
                    }
                }
                p { class: "text-sm opacity-60", style: "text-align: center;",
                    "Memorize the lit route — the map closes in a moment."
                }
                // Centered (not full-width) — the parent's items-center does it.
                button {
                    class: "{BTN} crt-btn-primary px-12 py-3 text-lg",
                    onclick: move |_| revealed.set(false),
                    "GO"
                }
            } else {
                // Navigate from memory. Footsteps show progress; tap the turns.
                p { class: "text-sm opacity-60", style: "text-align: center;",
                    "Run the route from memory. Step {step().min(path_len)} of {path_len}."
                }
                div { class: "flex gap-2 flex-wrap justify-center text-lg",
                    for i in 0..path_len {
                        span {
                            key: "{i}",
                            style: if i < step() {
                                "color: var(--phosphor); text-shadow: var(--glow);"
                            } else {
                                "color: var(--phosphor-dim); opacity: 0.4;"
                            },
                            if i < step() { "●" } else { "○" }
                        }
                    }
                }
                // A D-pad: the directions sit in their cardinal places so the
                // tap maps to the way you'd move. Corners are empty spacers.
                // Grid is styled inline (not just via classes): the Tailwind
                // Play CDN JIT-generates utilities from the DOM, so on a fresh
                // load the `grid grid-cols-3` classes aren't ready and the
                // buttons stack until a re-render. Classes stay for hosts that
                // do build them.
                div {
                    class: "grid grid-cols-3 gap-2",
                    style: "display: grid; grid-template-columns: repeat(3, 1fr); \
                        gap: 0.5rem; width: 11rem;",
                    div {}
                    button {
                        key: "up", class: "{BTN} py-3 text-xl",
                        onclick: move |_| tap(Dir::Up), "{Dir::Up.glyph()}"
                    }
                    div {}
                    button {
                        key: "left", class: "{BTN} py-3 text-xl",
                        onclick: move |_| tap(Dir::Left), "{Dir::Left.glyph()}"
                    }
                    // Phase-2 countdown, centered in the D-pad. Turns urgent in
                    // the final seconds; reaching 0 fails the run.
                    div {
                        key: "timer",
                        style: format_args!(
                            "display: flex; align-items: center; justify-content: center; \
                             font-size: 1.25rem; font-weight: 700; font-variant-numeric: tabular-nums; \
                             color: var(--phosphor); {}",
                            if nav_urgent {
                                "opacity: 1; text-shadow: 0 0 8px var(--phosphor);"
                            } else {
                                "opacity: 0.85;"
                            }
                        ),
                        "{nav_secs}"
                    }
                    button {
                        key: "right", class: "{BTN} py-3 text-xl",
                        onclick: move |_| tap(Dir::Right), "{Dir::Right.glyph()}"
                    }
                    div {}
                    button {
                        key: "down", class: "{BTN} py-3 text-xl",
                        onclick: move |_| tap(Dir::Down), "{Dir::Down.glyph()}"
                    }
                    div {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_grows_with_crowd_and_stays_in_bounds() {
        let small = grid_dims(1);
        let big = grid_dims(100);
        assert!(small.0 >= 4 && small.1 >= 4);
        assert!(big.0 <= 8 && big.1 <= 8);
        // Monotone: a packed crowd is never on a smaller grid than a thin one.
        assert!(grid_dims(40).0 >= grid_dims(4).0);
    }

    #[test]
    fn route_is_contiguous_in_bounds_and_reaches_the_exit() {
        for seed in 0..64 {
            let lvl = build_level(8, seed);
            assert_eq!(lvl.walked.first(), Some(&lvl.start));
            assert_eq!(lvl.walked.last(), Some(&lvl.exit));
            assert_eq!(lvl.exit.1, 0, "exit must be on the top row (seed {seed})");
            assert_eq!(lvl.path.len() + 1, lvl.walked.len());
            // Every cell is in bounds and one step from the previous.
            for w in &lvl.walked {
                assert!(w.0 < lvl.cols && w.1 < lvl.rows);
            }
            for (i, dir) in lvl.path.iter().enumerate() {
                let (c0, r0) = lvl.walked[i];
                let (c1, r1) = lvl.walked[i + 1];
                let (dc, dr) = dir.step();
                assert_eq!((c0 as i32 + dc, r0 as i32 + dr), (c1 as i32, r1 as i32));
            }
        }
    }

    #[test]
    fn obstacles_never_sit_on_the_route() {
        for seed in 0..64 {
            let lvl = build_level(12, seed);
            for o in &lvl.obstacles {
                assert!(!lvl.walked.contains(o), "crowd member on route (seed {seed})");
                assert!(o.0 < lvl.cols && o.1 < lvl.rows);
            }
        }
    }

    #[test]
    fn obstacle_count_is_crowd_size_capped_at_free_cells() {
        let lvl = build_level(6, 3);
        let free = lvl.cols * lvl.rows - lvl.walked.len();
        assert_eq!(lvl.obstacles.len(), 6.min(free));
    }

    #[test]
    fn same_seed_same_level_different_seed_differs() {
        assert_eq!(build_level(8, 7), build_level(8, 7));
        let base = build_level(8, 7);
        assert!((0..16).any(|s| build_level(8, s) != base));
    }

    #[test]
    fn dir_glyphs_and_steps_match() {
        assert_eq!(Dir::Up.step(), (0, -1));
        assert_eq!(Dir::Down.step(), (0, 1));
        assert_eq!(Dir::Left.step(), (-1, 0));
        assert_eq!(Dir::Right.step(), (1, 0));
        assert_eq!(Dir::Up.glyph(), "↑");
        assert_eq!(Dir::Left.glyph(), "←");
    }
}
