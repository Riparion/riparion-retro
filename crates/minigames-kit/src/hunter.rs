//! A shooting-gallery minigame: a hunter on the bottom row tracks a quarry that
//! bounces back and forth across the top, and fires straight up to bag it.
//!
//! The hunter slides left/right one cell per tap; the quarry ping-pongs along
//! the top row, its starting direction chosen by `seed`. The D-pad's top button
//! fires and the bottom button reloads: it's a single-shot rifle, so every shot
//! must be followed by a reload before the next. Ammo is finite — the quarry
//! bounces forever, but once the last round is spent on a miss the round is lost.
//!
//! Like the other minigames in this kit it's self-contained: it owns its timing
//! and input and reports a plain [`HunterResult`] through `on_complete`, leaving
//! scoring to the host. The quarry's motion is a pure function of elapsed ticks
//! and the seed, so the component adds no nondeterminism of its own.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use retro_kit::rng::GameRng;
use retro_kit::theme::{BTN, SCREEN_CENTERED};

/// Granularity of the motion/reload clock, in milliseconds (~50 fps).
pub const TICK_MS: u32 = 20;

/// Ticks the bullet tracer stays visible after a shot (~120ms at `TICK_MS`).
/// A terminal shot defers ending the round by this long so the bullet is seen.
const TRACER_TICKS: u32 = 6;

/// The outcome of one Hunter round, handed to the host's `on_complete`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HunterResult {
    /// Whether a shot landed on the quarry (the round was won).
    pub hit: bool,
    /// Rounds fired before the round ended (hit, or ammo exhausted).
    pub shots_fired: usize,
}

/// Column the quarry occupies after `elapsed_ms`, bouncing across `[0, cols-1]`.
///
/// A triangle wave: it walks one column per `step_ms`, reverses at each edge,
/// and repeats. `start_right` mirrors the path about the center so the quarry
/// enters from the right edge moving left instead of the left edge moving right.
/// Pure and deterministic — the component's only source of motion.
pub fn quarry_col(elapsed_ms: f64, step_ms: f64, cols: usize, start_right: bool) -> usize {
    if cols <= 1 || step_ms <= 0.0 {
        return 0;
    }
    let span = (cols - 1) as f64;
    let steps = elapsed_ms / step_ms;
    // Fold the step count into a triangle over [0, span]: 0..span then span..0.
    let phase = steps.rem_euclid(2.0 * span);
    let tri = if phase <= span { phase } else { 2.0 * span - phase };
    let col = tri.round() as usize;
    let col = col.min(cols - 1);
    if start_right {
        cols - 1 - col
    } else {
        col
    }
}

/// A shooting-gallery minigame. Mount it with a `key` that changes per encounter
/// (so the round and its seed reset) and wire `on_complete`:
///
/// ```rust,ignore
/// Hunter {
///     key: "{round}",
///     prompt: "Bag the buck —",
///     hunter_icon: "🔫",
///     hunted_icon: "🦌",
///     ammo: 5,
///     seed,
///     on_complete: move |r: HunterResult| { /* score it */ },
/// }
/// ```
#[component]
pub fn Hunter(
    /// Line shown above the range ("Bag the buck —").
    prompt: String,
    /// Glyph for the hunter on the bottom row. Defaults to a portable `@`; pass
    /// an emoji (e.g. "🔫") on hosts with a color-emoji font.
    #[props(default = String::from("@"))]
    hunter_icon: String,
    /// Glyph for the quarry sweeping the top row. Defaults to a portable `d`.
    #[props(default = String::from("d"))]
    hunted_icon: String,
    /// Range width in cells. Wider = the quarry covers more ground per pass.
    #[props(default = 7)]
    cols: usize,
    /// Range height in cells (vertical gap between hunter and quarry).
    #[props(default = 5)]
    rows: usize,
    /// Total rounds for the encounter (the chambered one counts). When the last
    /// is spent on a miss, the round is lost.
    #[props(default = 5)]
    ammo: usize,
    /// Milliseconds the quarry spends crossing one cell. Lower = faster/harder.
    #[props(default = 320)]
    step_ms: u32,
    /// Milliseconds to chamber the next round after firing.
    #[props(default = 800)]
    reload_ms: u32,
    /// Seed for the hunter's start column and the quarry's starting direction.
    /// Same seed → same setup; vary it per encounter.
    #[props(default = 0)]
    seed: u64,
    /// Called once, when the quarry is hit or the ammo runs out.
    on_complete: EventHandler<HunterResult>,
) -> Element {
    // Layout floors, so a careless caller can't produce a degenerate grid.
    let cols = cols.max(2);
    let rows = rows.max(2);

    // Seeded setup, pinned once per mount: where the hunter starts and which way
    // the quarry first moves.
    let (start_col, start_right) = use_hook(move || {
        let mut rng = GameRng::from_seed(seed);
        let col = rng.ri(cols as i64) as usize;
        let right = rng.one_in(2.0);
        (col, right)
    });

    let mut ticks = use_signal(|| 0u32);
    let mut hunter_pos = use_signal(move || start_col);
    let mut loaded = use_signal(|| true);
    let mut reloading = use_signal(|| false);
    let mut reload_done_tick = use_signal(|| 0u32);
    let mut ammo_left = use_signal(move || ammo.max(1));
    let mut shots_fired = use_signal(|| 0usize);
    let mut fired_tick = use_signal(|| None::<u32>);
    let mut done = use_signal(|| false);
    // Outcome of a terminal shot, locked at fire time and reported by the tick
    // loop once the tracer finishes (so a winning bullet isn't cut short).
    let mut pending = use_signal(|| None::<HunterResult>);

    // The clock: advance ticks (which drive the quarry) and finish a reload when
    // its timer elapses. Runs until the round ends.
    use_future(move || async move {
        loop {
            if done() {
                break;
            }
            TimeoutFuture::new(TICK_MS).await;
            ticks += 1;
            if reloading() && ticks() >= reload_done_tick() {
                reloading.set(false);
                loaded.set(true);
            }
            // A terminal shot's outcome was locked at fire time; report it once
            // the tracer has flown so the bullet is seen before we hand off.
            if let (Some(res), Some(ft)) = (pending(), fired_tick()) {
                if ticks() >= ft + TRACER_TICKS {
                    done.set(true);
                    on_complete.call(res);
                }
            }
        }
    });

    // The quarry's current column, derived from elapsed ticks.
    let elapsed_ms = ticks() as f64 * TICK_MS as f64;
    let quarry = quarry_col(elapsed_ms, step_ms as f64, cols, start_right);
    let hunter = hunter_pos();

    // A bullet dot streaks up the hunter's column for a few ticks after firing —
    // from just above the rifle to the top row. A terminal shot defers ending the
    // round until the tracer finishes (see the tick loop), so the bullet is seen
    // on a hit too rather than vanishing the instant the round hands off.
    let bullet = match fired_tick() {
        Some(t) if !done() => {
            let since = ticks().saturating_sub(t);
            if since < TRACER_TICKS {
                // since 0 → just above the hunter; near TRACER_TICKS → the top row.
                let frac = since as f64 / TRACER_TICKS as f64;
                let row = ((1.0 - frac) * (rows as f64 - 2.0)).round() as usize;
                Some((hunter, row.min(rows - 2)))
            } else {
                None
            }
        }
        _ => None,
    };

    let reload_ticks = (reload_ms / TICK_MS).max(1);
    let can_fire = loaded() && !reloading() && !done();
    let can_reload =
        !loaded() && !reloading() && ammo_left() > 0 && !done() && pending().is_none();

    // Fire up the hunter's column. A hit when the quarry shares the column ends
    // the round; otherwise the chamber empties, and spending the last round on a
    // miss loses it.
    let mut fire = move || {
        if !can_fire {
            return;
        }
        loaded.set(false);
        fired_tick.set(Some(ticks()));
        ammo_left -= 1;
        shots_fired += 1;
        // Lock the outcome at the moment of firing. A hit, or spending the last
        // round on a miss, ends the round — but defer that to the tick loop so
        // the bullet finishes its flight first.
        let hit = quarry == hunter;
        if hit || ammo_left() == 0 {
            pending.set(Some(HunterResult {
                hit,
                shots_fired: shots_fired(),
            }));
        }
    };

    let mut reload = move || {
        if !can_reload {
            return;
        }
        reloading.set(true);
        reload_done_tick.set(ticks() + reload_ticks);
    };

    let mut step_hunter = move |delta: i64| {
        if done() {
            return;
        }
        let next = (hunter_pos() as i64 + delta).clamp(0, cols as i64 - 1) as usize;
        hunter_pos.set(next);
    };

    // D-pad button styles, fully inlined. The Tailwind Play CDN applies the
    // `py-3`/`text-sm`/`text-xl` utilities only once it catches up (on the first
    // interaction), which would otherwise grow the buttons and shift the
    // vertically-centered game. Inlining padding + font-size + line-height pins
    // the height from the first paint; opacity dims a currently-unavailable one.
    let dim = |enabled: bool| if enabled { "opacity: 1;" } else { "opacity: 0.35;" };
    let fire_style = format!("{} padding: 0.75rem; font-size: 0.875rem; line-height: 1;", dim(can_fire));
    let reload_style = format!("{} padding: 0.75rem; font-size: 0.875rem; line-height: 1;", dim(can_reload));
    let move_style = "padding: 0.75rem; font-size: 1.25rem; line-height: 1;";

    rsx! {
        // Inline layout mirrors the other minigames: some hosts (the Tailwind
        // Play CDN) don't emit the flex utilities for this dynamic DOM, so the
        // classes alone wouldn't center it.
        div {
            class: "{SCREEN_CENTERED} gap-2 items-center",
            style: "display: flex; flex-direction: column; align-items: center; \
                justify-content: center; flex: 1; gap: 0.5rem; padding: 1rem; \
                width: 100%; max-width: 28rem; margin: 0 auto;",
            p {
                class: "text-lg opacity-80",
                // Size/opacity inlined so the prompt doesn't resize when the
                // Tailwind Play CDN catches up after the first interaction.
                style: "text-align: center; font-size: 1.125rem; opacity: 0.8;",
                "{prompt}"
            }

            // The range: a dim lattice with the quarry up top and the hunter on
            // the bottom row. Styled inline so the core visual needs no utilities.
            div {
                style: "display: grid; width: 100%; max-width: 19rem; \
                    grid-template-columns: repeat({cols}, 1fr); gap: 2px; \
                    padding: 4px; border: 1px solid var(--phosphor-dim); border-radius: 4px; \
                    background: rgba(20, 95, 40, 0.10);",
                for r in 0..rows {
                    for c in 0..cols {
                        {
                            let is_hunter = c == hunter && r == rows - 1;
                            let is_quarry = c == quarry && r == 0;
                            let is_bullet = bullet == Some((c, r));
                            let glyph = if is_hunter {
                                hunter_icon.clone()
                            } else if is_quarry {
                                hunted_icon.clone()
                            } else if is_bullet {
                                "•".to_string()
                            } else {
                                String::new()
                            };
                            // The bullet streaks with a tight glow; hunter and
                            // quarry pop; empty cells stay dim. text-shadow is set
                            // in every branch — Dioxus patches style props one by
                            // one and won't drop an omitted one, so a past glow
                            // would otherwise linger as the dot moves on.
                            let glyph_style = if is_bullet {
                                "color: var(--phosphor); text-shadow: 0 0 6px var(--phosphor);"
                            } else if is_hunter || is_quarry {
                                "color: var(--phosphor); text-shadow: var(--glow);"
                            } else {
                                "color: var(--phosphor-dim); text-shadow: none;"
                            };
                            rsx! {
                                div {
                                    key: "{r}-{c}",
                                    // Cells are a flat dim lane — no per-cell glow,
                                    // so nothing of a shot is left behind once the
                                    // bullet dot has passed through. line-height +
                                    // overflow:hidden pin the cell height to its
                                    // aspect ratio so a glyph can't grow the row and
                                    // make the grid jitter as the dot moves through.
                                    style: "display: flex; align-items: center; justify-content: center; \
                                        aspect-ratio: 1; font-size: 1.35rem; font-weight: 700; \
                                        line-height: 1; overflow: hidden; \
                                        border-radius: 3px; border: 1px solid transparent; \
                                        background: rgba(51, 255, 102, 0.04);",
                                    span { style: "{glyph_style}", "{glyph}" }
                                }
                            }
                        }
                    }
                }
            }

            // Between the range and the D-pad: the ammo indicator — filled dots
            // for rounds still in the bag, hollow for spent — which flips to a
            // "Reloading…" notice while the next round is being chambered.
            div {
                class: "text-sm",
                // Fixed height (not min-height) with line-height:1 + overflow
                // hidden: the dot glyphs render taller than the "Reloading…"
                // text, so a min-height would let the line grow and shrink as the
                // two modes swap — shifting the vertically-centered game. Pinning
                // the height keeps it still. Font-size inlined so it's stable
                // whether or not the Tailwind Play CDN has caught up.
                style: "display: flex; align-items: center; justify-content: center; \
                    gap: 2px; height: 1.5rem; line-height: 1; overflow: hidden; \
                    font-size: 0.875rem; color: var(--phosphor); text-shadow: var(--glow);",
                // DIAGNOSTIC: status swap disabled — always render the dots so the
                // line's content never changes mode. If the shift is gone, the
                // dots↔"Reloading…" swap was the cause.
                for i in 0..ammo.max(1) {
                    span {
                        key: "{i}",
                        style: if i < ammo_left() { "opacity: 1;" } else { "opacity: 0.3;" },
                        if i < ammo_left() { "●" } else { "○" }
                    }
                }
            }

            // A D-pad: FIRE on top, RELOAD on the bottom, move with left/right.
            // Grid is styled inline (not just via classes): the Tailwind Play
            // CDN JIT-generates utilities from the DOM, so on a fresh load the
            // `grid grid-cols-3` classes aren't ready yet and the buttons stack
            // until a re-render. The classes stay for hosts that do build them.
            div {
                class: "grid grid-cols-3 gap-2",
                style: "display: grid; grid-template-columns: repeat(3, 1fr); \
                    gap: 0.5rem; width: 11rem;",
                div {}
                button {
                    key: "fire", class: "{BTN} py-3 text-sm",
                    style: "{fire_style}",
                    disabled: !can_fire,
                    onclick: move |_| fire(),
                    "FIRE"
                }
                div {}
                button {
                    key: "left", class: "{BTN} py-3 text-xl",
                    style: "{move_style}",
                    onclick: move |_| step_hunter(-1),
                    "←"
                }
                div {}
                button {
                    key: "right", class: "{BTN} py-3 text-xl",
                    style: "{move_style}",
                    onclick: move |_| step_hunter(1),
                    "→"
                }
                div {}
                button {
                    key: "reload", class: "{BTN} py-3 text-sm",
                    style: "{reload_style}",
                    disabled: !can_reload,
                    onclick: move |_| reload(),
                    "RELOAD"
                }
                div {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarry_stays_in_bounds_over_a_long_span() {
        let cols = 7;
        for i in 0..2000 {
            let col = quarry_col(i as f64 * 17.0, 320.0, cols, false);
            assert!(col < cols, "col {col} out of bounds at step {i}");
        }
    }

    #[test]
    fn quarry_reaches_both_edges_and_reverses() {
        let cols = 6;
        let step_ms = 100.0;
        // Sample one full back-and-forth (period = 2*(cols-1) steps).
        let cols_seen: Vec<usize> = (0..=2 * (cols - 1))
            .map(|s| quarry_col(s as f64 * step_ms, step_ms, cols, false))
            .collect();
        assert_eq!(cols_seen.first(), Some(&0), "starts at the left edge");
        assert!(cols_seen.contains(&(cols - 1)), "reaches the right edge");
        assert_eq!(cols_seen.last(), Some(&0), "returns to the left edge");
        // Monotone up to the far edge, then monotone back — no wrap jump.
        for w in cols_seen.windows(2) {
            assert!(
                w[1].abs_diff(w[0]) <= 1,
                "quarry jumped from {} to {}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn start_right_mirrors_start_left() {
        let cols = 7;
        let step_ms = 90.0;
        for s in 0..50 {
            let t = s as f64 * step_ms;
            let left = quarry_col(t, step_ms, cols, false);
            let right = quarry_col(t, step_ms, cols, true);
            assert_eq!(right, cols - 1 - left, "not a mirror at step {s}");
        }
    }
}
