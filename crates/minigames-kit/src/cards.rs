//! Shared deck primitives and the flip-on-mount card for the kit's card games
//! (Faro, Vingt-et-Un). Mirrors the [`crate::grid`] module: it holds the pieces
//! the card games would otherwise each copy — the rank constants, the card-face
//! styling, the Fisher–Yates shuffle, the stake-progression end check, and the
//! self-contained deal flip — so a new card game adds only its own rules.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use retro_kit::rng::GameRng;

/// The thirteen ranks, ace-low index through king (index 0..13).
pub const RANKS: usize = 13;
/// A full deck: four suits of each rank.
pub const DECK: usize = 52;

/// On-screen rank labels, indexed 0..13.
pub const RANK_LABELS: [&str; RANKS] =
    ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];

/// Inline card-face styling (border, rounded, faint phosphor fill, 3:4 portrait),
/// applied directly so a host needs no card-game-specific CSS compiled — the same
/// inline-style convention the kit's grid minigames use ([`crate::grid`]).
pub const CARD_FACE_STYLE: &str = "border:1px solid var(--phosphor-dim);border-radius:3px;\
background:rgba(51,255,102,.04);display:flex;flex-direction:column;align-items:center;\
justify-content:center;aspect-ratio:3/4;";

/// Fisher–Yates shuffle in place, drawing swaps from an already-seeded `rng`.
/// Each card game builds its own deck contents (ranks-only for Faro, full card
/// indices for Vingt-et-Un) and seeds the rng how it likes, then calls this.
pub fn shuffle(deck: &mut [u8], rng: &mut GameRng) {
    for i in (1..deck.len()).rev() {
        let j = rng.ri(i as i64 + 1) as usize;
        deck.swap(i, j);
    }
}

/// Whether a card-gambling session ends after a round/turn settles: the stake is
/// busted, the target is reached, or the round allowance is spent. Shared by the
/// card games so the win/bust/exhaustion boundary stays identical.
pub fn session_over(stake: i64, target: i64, played: usize, rounds_total: usize) -> bool {
    stake <= 0 || stake >= target || played >= rounds_total
}

/// A card slot that flips in on mount: an inline transform that transitions from
/// edge-on to face-on once mounted, wrapping whatever face the caller renders as
/// `children`. The flip is self-contained — no global CSS or keyframes leak into
/// the host document — so a card replays it whenever the caller gives it a fresh
/// `key` (per turn in Faro, per dealt card in Vingt-et-Un).
#[component]
pub fn FlipCard(
    /// CSS width for the card box (e.g. `"100%"` to fill a column, `"3rem"` fixed).
    #[props(default = "100%".to_string())]
    width: String,
    /// The card face to render inside the flipping box.
    children: Element,
) -> Element {
    let mut flipped = use_signal(|| false);
    use_future(move || async move {
        TimeoutFuture::new(20).await;
        flipped.set(true);
    });
    let (transform, opacity) = if flipped() {
        ("none", "1")
    } else {
        ("rotateY(85deg) scale(0.7)", "0")
    };
    let card_style = format!(
        "{CARD_FACE_STYLE}width:{width};transition:transform 320ms ease-out,opacity 320ms ease-out;\
         transform:{transform};opacity:{opacity};"
    );
    rsx! {
        div { style: "{card_style}", {children} }
    }
}
