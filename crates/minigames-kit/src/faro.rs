//! Faro — the gambling game that ran the Old West, distilled to its decisions.
//!
//! Faro is a banking card game played against a "layout" of the thirteen ranks
//! (suits never matter — only rank does). Each **turn** the dealer pulls two
//! cards from the box: the **losing** card first, then the **winning** card.
//! Chips on the losing rank are raked by the bank; chips on the winning rank pay
//! even money. Betting a rank to *lose* instead is called **coppering** it (a
//! copper token was laid on the stack). When both cards of a turn are the same
//! rank it's a **split**, and the bank takes half of any bet on it.
//!
//! The only real skill in faro was the **case keeper**: tracking how many of
//! each rank had already been turned, so late in the deck you could bet the
//! ranks whose remaining odds had swung your way. This minigame keeps exactly
//! that — a live remaining-count for every rank — and asks you to grow a stake
//! to a target before the deck (or your turn allowance) runs out, without going
//! bust. Everything derives from `seed` (the shuffled deck), so a run is
//! deterministic and replayable, and the scoring core is pure and unit-tested.
//!
//! The host turns [`FaroResult`] into whatever it needs (kaintuck folds a saloon
//! night at Natchez into the player's purse, for instance). No canvas or WebGL:
//! the cards are styled DOM, the deal a self-contained CSS flip.

use dioxus::prelude::*;

use retro_kit::format::fmt_dollars_compact;
use retro_kit::rng::GameRng;
use retro_kit::theme::{BTN, BTN_PRIMARY, BTN_WIDE};

use crate::cards::{session_over, shuffle, FlipCard, CARD_FACE_STYLE, DECK, RANKS, RANK_LABELS};

/// Turns dealt from one deck: the first card is burned ("soda"), then 25 paired
/// turns are pulled, leaving the last card ("hock") in the box.
pub const TURNS: usize = 25;

/// The outcome of a faro session, handed to the host's `on_complete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaroResult {
    /// Whether the player reached the target stake.
    pub won: bool,
    /// The stake left when the session ended (busted = 0).
    pub final_stake: i64,
    /// How many turns were dealt before the session ended.
    pub turns_played: usize,
}

/// The two cards pulled on a turn: the losing (banker's) card and the winning
/// (player's) card, as ranks 0..13.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Turn {
    /// The rank that loses this turn (raked by the bank).
    pub loser: u8,
    /// The rank that wins this turn (pays even money).
    pub winner: u8,
}

impl Turn {
    /// A split: both cards are the same rank. The bank takes half of any bet
    /// riding on that rank.
    pub fn split(&self) -> bool {
        self.loser == self.winner
    }
}

/// One placed bet: `amount` chips on `rank`, betting it to win — or to lose if
/// `copper` is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bet {
    /// The rank the chips sit on (0..13).
    pub rank: u8,
    /// Chips wagered (in the host's money units).
    pub amount: i64,
    /// Coppered: betting the rank to *lose* rather than win.
    pub copper: bool,
}

/// A shuffled deck as ranks 0..13 (four of each), seeded so the same `seed`
/// always deals the same cards. Fisher–Yates, the same idiom the other
/// minigames use for deterministic shuffles.
pub fn dealt_deck(seed: u64) -> Vec<u8> {
    // 0,1,…,12,0,1,…,12,… over 52 slots is exactly four of each rank.
    let mut deck: Vec<u8> = (0..DECK).map(|i| (i % RANKS) as u8).collect();
    let mut rng = GameRng::from_seed(seed);
    shuffle(&mut deck, &mut rng);
    deck
}

/// The two cards pulled on turn `t` (0-based). `deck[0]` is the burned soda, so
/// turn 0 is `deck[1]` (loser) and `deck[2]` (winner), and so on.
pub fn turn_at(deck: &[u8], t: usize) -> Turn {
    let lo = 1 + 2 * t;
    Turn {
        loser: deck[lo],
        winner: deck[lo + 1],
    }
}

/// Net stake change from settling `bets` against one `turn`. Positive means the
/// player gains. Untouched ranks return their chips (zero delta); on a split the
/// bank takes half of any bet on the split rank, win or copper alike.
pub fn settle(bets: &[Bet], turn: Turn) -> i64 {
    bets.iter().map(|b| settle_one(*b, turn)).sum()
}

/// Settle a single bet — the per-rank core of [`settle`].
fn settle_one(b: Bet, turn: Turn) -> i64 {
    let on_winner = b.rank == turn.winner;
    let on_loser = b.rank == turn.loser;
    if turn.split() {
        // loser == winner: any bet on the split rank loses half, rounded down.
        if on_winner {
            -(b.amount / 2)
        } else {
            0
        }
    } else if b.copper {
        // Coppered: the rank winning the turn now costs you, losing pays.
        if on_loser {
            b.amount
        } else if on_winner {
            -b.amount
        } else {
            0
        }
    } else if on_winner {
        b.amount
    } else if on_loser {
        -b.amount
    } else {
        0
    }
}

/// How many of each rank remain in the box after `turns_played` turns (the
/// burned soda counts as consumed). The case keeper, indexed 0..13.
pub fn remaining_counts(deck: &[u8], turns_played: usize) -> [u8; RANKS] {
    let consumed = (1 + 2 * turns_played).min(deck.len());
    let mut counts = [0u8; RANKS];
    for &r in &deck[consumed..] {
        counts[r as usize] += 1;
    }
    counts
}

/// A self-contained faro table. Mount it (with a `key` that changes per session
/// so the deck and stake reset) and wire `on_complete`:
///
/// ```rust,ignore
/// Faro {
///     key: "{night}",
///     prompt: "Under-the-Hill at Natchez. The bank is open —",
///     seed,                  // drawn from the host's RNG
///     starting_stake: 200,
///     target_stake: 500,
///     on_complete: move |r: FaroResult| { /* fold into the purse */ },
/// }
/// ```
#[component]
pub fn Faro(
    /// Line shown above the table (the situation/flavor).
    prompt: String,
    /// Seed for the deck shuffle. Same seed → same deal (deterministic).
    seed: u64,
    /// Chips the player sits down with.
    starting_stake: i64,
    /// Stake to reach to win the session.
    target_stake: i64,
    /// Maximum turns to deal; clamped to a deck's worth ([`TURNS`]).
    #[props(default = TURNS)]
    max_turns: usize,
    /// Chips added per tap on a rank.
    #[props(default = 25)]
    min_bet: i64,
    /// Called once, when the session ends (target reached, bust, or turns spent).
    on_complete: EventHandler<FaroResult>,
) -> Element {
    let turns_total = max_turns.min(TURNS).max(1);
    // Defensive prop clamp (the kit convention, cf. hunter's cols/ammo): a tap
    // must always be able to place at least one bet, so cap min_bet to the
    // opening stake — otherwise min_bet > starting_stake soft-locks the table.
    let min_bet = min_bet.clamp(1, starting_stake.max(1));

    let deck = use_signal(move || dealt_deck(seed));
    let mut stake = use_signal(move || starting_stake);
    let mut turn_idx = use_signal(|| 0usize);
    // This turn's placements: chips per rank, and whether each is coppered.
    let mut wagers = use_signal(|| [0i64; RANKS]);
    let mut copper = use_signal(|| [false; RANKS]);
    // Placement mode: tapping a rank bets it to lose (copper) when true.
    let mut copper_mode = use_signal(|| false);
    // Once dealt, holds the revealed turn and its net delta; bets are locked.
    let mut revealed = use_signal(|| None::<(Turn, i64)>);

    let placed: i64 = wagers.read().iter().sum();
    let is_revealed = revealed.read().is_some();
    // The case keeper: how many of each rank remain in the box. Once a turn is
    // revealed its two cards are out, so count them consumed (turn_idx + 1).
    // Memoized on turn_idx/revealed so taps and toggles within a turn don't
    // re-scan the deck on every render.
    let counts_memo = use_memo(move || {
        let played = turn_idx() + revealed.read().is_some() as usize;
        remaining_counts(&deck.read(), played)
    });
    let counts = counts_memo();

    rsx! {
        div { class: "flex-1 flex flex-col p-3 max-w-md w-full mx-auto gap-3",
            p { class: "text-center text-sm opacity-80", "{prompt}" }

            // Status: stake / target / turn.
            div { class: "grid grid-cols-3 gap-2 text-center",
                div { class: "crt-panel py-1",
                    div { class: "chip-label", "STAKE" }
                    div { class: "text-lg", "{fmt_dollars_compact(stake() as f64)}" }
                }
                div { class: "crt-panel py-1",
                    div { class: "chip-label", "TARGET" }
                    div { class: "text-lg", "{fmt_dollars_compact(target_stake as f64)}" }
                }
                div { class: "crt-panel py-1",
                    div { class: "chip-label", "TURN" }
                    div { class: "text-lg", "{(turn_idx() + 1).min(turns_total)}/{turns_total}" }
                }
            }

            // Dealt cards, or a hint while betting.
            match revealed() {
                Some((t, delta)) => rsx! {
                    div { class: "flex items-stretch justify-center gap-4",
                        DealtCard { key: "{turn_idx()}-lose", rank: t.loser, label: "LOSES", danger: true }
                        DealtCard { key: "{turn_idx()}-win", rank: t.winner, label: "WINS", danger: false }
                    }
                    div { class: "text-center text-sm",
                        if t.split() {
                            span { class: "chip-danger", "SPLIT — bank takes half. " }
                        }
                        match delta {
                            d if d > 0 => rsx! { span { "You win {fmt_dollars_compact(d as f64)}." } },
                            d if d < 0 => rsx! { span { class: "chip-danger", "You lose {fmt_dollars_compact((-d) as f64)}." } },
                            _ => rsx! { span { class: "opacity-70", "No action on your bets." } },
                        }
                    }
                },
                None => rsx! {
                    p { class: "text-center text-xs opacity-60",
                        "Tap ranks to stake "
                        "{fmt_dollars_compact(min_bet as f64)}"
                        " each. Copper a rank to bet it loses. Then deal."
                    }
                },
            }

            // The layout: thirteen ranks. Tap to bet; badges show your chips,
            // a copper marker, and the case keeper's remaining count.
            div { class: "grid grid-cols-5 gap-1",
                for r in 0..RANKS {
                    {
                        let wager = wagers.read()[r];
                        let is_copper = copper.read()[r];
                        rsx! {
                            button {
                                key: "{r}",
                                class: "py-1 relative",
                                disabled: is_revealed,
                                style: if wager > 0 { "{CARD_FACE_STYLE}border-color:var(--phosphor);" } else { "{CARD_FACE_STYLE}" },
                                onclick: move |_| {
                                    if is_revealed { return; }
                                    let mode = copper_mode();
                                    let cur: i64 = wagers.read().iter().sum();
                                    if cur + min_bet > stake() { return; }
                                    wagers.write()[r] += min_bet;
                                    copper.write()[r] = mode;
                                },
                                div { class: "text-base leading-none",
                                    "{RANK_LABELS[r]}"
                                    span { class: "opacity-60", "♠" }
                                }
                                div { class: "chip-label", "{counts[r]} left" }
                                if wager > 0 {
                                    div {
                                        class: if is_copper { "chip-danger text-xs" } else { "text-xs" },
                                        if is_copper { "⊘" }
                                        "{fmt_dollars_compact(wager as f64)}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Controls.
            if is_revealed {
                {
                    let played = turn_idx() + 1;
                    let over = session_over(stake(), target_stake, played, turns_total);
                    // A win means reaching the target while still solvent — a bust
                    // (stake 0) never counts, even if a host set a non-positive target.
                    let won = stake() > 0 && stake() >= target_stake;
                    rsx! {
                        button {
                            class: "{BTN_PRIMARY}",
                            onclick: move |_| {
                                if over {
                                    on_complete.call(FaroResult {
                                        won,
                                        final_stake: stake().max(0),
                                        turns_played: played,
                                    });
                                } else {
                                    turn_idx += 1;
                                    wagers.set([0; RANKS]);
                                    copper.set([false; RANKS]);
                                    revealed.set(None);
                                }
                            },
                            if over {
                                if won { "Cash out — you beat the bank" }
                                else if stake() <= 0 { "Tapped out — leave the table" }
                                else { "The deck is spent — leave the table" }
                            } else { "Next turn" }
                        }
                    }
                }
            } else {
                div { class: "grid grid-cols-2 gap-2",
                    button {
                        class: if copper_mode() { "crt-btn chip-danger" } else { "{BTN}" },
                        onclick: move |_| copper_mode.toggle(),
                        if copper_mode() { "Coppering ⊘" } else { "Bet to win" }
                    }
                    button {
                        class: "{BTN}",
                        disabled: placed == 0,
                        onclick: move |_| {
                            wagers.set([0; RANKS]);
                            copper.set([false; RANKS]);
                        },
                        "Clear"
                    }
                }
                button {
                    class: "{BTN_PRIMARY}",
                    onclick: move |_| {
                        if is_revealed { return; }
                        let t = turn_at(&deck.read(), turn_idx());
                        let w = wagers.read();
                        let c = copper.read();
                        let bets: Vec<Bet> = (0..RANKS)
                            .filter(|&r| w[r] > 0)
                            .map(|r| Bet { rank: r as u8, amount: w[r], copper: c[r] })
                            .collect();
                        let delta = settle(&bets, t);
                        stake += delta;
                        revealed.set(Some((t, delta)));
                    },
                    if placed > 0 { "Deal — {fmt_dollars_compact(placed as f64)} at stake" } else { "Deal (sit this one out)" }
                }
                // Walk away between turns with the chips you've still got — the
                // host folds them back into the purse just as a cash-out would.
                button {
                    class: "{BTN_WIDE}",
                    onclick: move |_| {
                        on_complete.call(FaroResult {
                            won: stake() > 0 && stake() >= target_stake,
                            final_stake: stake().max(0),
                            turns_played: turn_idx(),
                        });
                    },
                    "Leave the table — keep your chips"
                }
            }
        }
    }
}

/// One revealed card in a turn: a styled face with the rank and a WINS/LOSES
/// label, dealt in with the shared [`FlipCard`] flip (self-contained, so no
/// global CSS leaks; remounts per turn via its `key`, replaying the flip).
#[component]
fn DealtCard(rank: u8, label: String, danger: bool) -> Element {
    let pip_class = if danger { "chip-danger" } else { "" };
    rsx! {
        div { class: "flex flex-col items-center gap-1 w-20",
            FlipCard {
                div { class: "text-3xl leading-none {pip_class}",
                    "{RANK_LABELS[rank as usize]}"
                    span { class: "opacity-70", "♠" }
                }
            }
            div { class: "chip-label {pip_class}", "{label}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bet(rank: u8, amount: i64, copper: bool) -> Bet {
        Bet { rank, amount, copper }
    }

    #[test]
    fn deck_is_four_of_each_rank() {
        let deck = dealt_deck(12345);
        assert_eq!(deck.len(), DECK);
        let mut counts = [0u8; RANKS];
        for &r in &deck {
            assert!((r as usize) < RANKS);
            counts[r as usize] += 1;
        }
        assert!(counts.iter().all(|&c| c == 4));
    }

    #[test]
    fn deck_is_deterministic_per_seed() {
        assert_eq!(dealt_deck(7), dealt_deck(7));
        assert!((0..32).any(|s| dealt_deck(s) != dealt_deck(7)));
    }

    #[test]
    fn turn_skips_the_soda_and_advances_by_two() {
        let deck: Vec<u8> = (0..DECK as u8).collect();
        // deck[0] is burned; turn 0 = (deck[1], deck[2]).
        assert_eq!(turn_at(&deck, 0), Turn { loser: 1, winner: 2 });
        assert_eq!(turn_at(&deck, 1), Turn { loser: 3, winner: 4 });
        assert_eq!(turn_at(&deck, 24), Turn { loser: 49, winner: 50 });
    }

    #[test]
    fn settle_win_lose_and_no_action() {
        let turn = Turn { loser: 3, winner: 8 };
        assert_eq!(settle(&[bet(8, 100, false)], turn), 100); // on winner
        assert_eq!(settle(&[bet(3, 100, false)], turn), -100); // on loser
        assert_eq!(settle(&[bet(5, 100, false)], turn), 0); // uninvolved
    }

    #[test]
    fn copper_inverts_win_and_lose() {
        let turn = Turn { loser: 3, winner: 8 };
        assert_eq!(settle(&[bet(3, 100, true)], turn), 100); // coppered loser pays
        assert_eq!(settle(&[bet(8, 100, true)], turn), -100); // coppered winner costs
        assert_eq!(settle(&[bet(5, 100, true)], turn), 0);
    }

    #[test]
    fn split_takes_half_win_or_copper() {
        let turn = Turn { loser: 6, winner: 6 };
        assert_eq!(settle(&[bet(6, 100, false)], turn), -50);
        assert_eq!(settle(&[bet(6, 100, true)], turn), -50);
        assert_eq!(settle(&[bet(6, 101, false)], turn), -50); // rounds down
        assert_eq!(settle(&[bet(2, 100, false)], turn), 0); // off the split
    }

    #[test]
    fn settle_sums_across_bets() {
        let turn = Turn { loser: 3, winner: 8 };
        let bets = [bet(8, 100, false), bet(3, 40, false), bet(1, 25, false)];
        assert_eq!(settle(&bets, turn), 100 - 40);
    }

    #[test]
    fn remaining_counts_shrink_as_turns_play() {
        let deck = dealt_deck(99);
        let start = remaining_counts(&deck, 0);
        assert_eq!(start.iter().map(|&c| c as usize).sum::<usize>(), DECK - 1); // soda burned
        let later = remaining_counts(&deck, 5);
        assert_eq!(later.iter().map(|&c| c as usize).sum::<usize>(), DECK - 1 - 10);
        // The full deal leaves exactly the hock card.
        let end = remaining_counts(&deck, TURNS);
        assert_eq!(end.iter().map(|&c| c as usize).sum::<usize>(), 1);
    }
}
