//! Vingt-et-Un — twenty-one, the ancestor of blackjack, as it ran the tables
//! Under-the-Hill at Natchez.
//!
//! Vingt-et-un entered the United States through New Orleans and rode up the
//! Mississippi with the keelboat trade; alongside faro it was the game the
//! professionals actually preferred, where a steady banker held the edge over a
//! shifting crowd of flatboatmen. The rules here are the period ones, stripped
//! to the one decision that matters: each **round** you lay a stake, take two
//! cards against the dealer's, and choose whether to **draw** (hit) toward
//! twenty-one or **stand**. Go over and you bust on the spot. The dealer then
//! plays a fixed hand — drawing until reaching seventeen — and the closer hand to
//! twenty-one takes the stake. A two-card twenty-one is a **vingt-et-un** (a
//! "natural") and pays a half again.
//!
//! Unlike faro, the skill here isn't counting the box — it's the draw decision
//! against the dealer's up-card, so each round is dealt from its own fresh,
//! deterministic shuffle (everything derives from `seed` and the round number,
//! so a run is replayable and the scoring core is pure and unit-tested). The
//! goal, as in faro, is to grow a stake to a target before your round allowance
//! runs out, without busting your purse.
//!
//! The host turns [`VingtUnResult`] into whatever it needs. No canvas or WebGL:
//! the cards are styled DOM, the deal a self-contained CSS flip — the same
//! convention [`crate::faro`] uses, so a host needs no game-specific CSS compiled.

use dioxus::prelude::*;

use retro_kit::format::fmt_dollars_compact;
use retro_kit::rng::GameRng;
use retro_kit::theme::{BTN, BTN_PRIMARY};

use crate::cards::{session_over, shuffle, FlipCard, DECK, RANKS, RANK_LABELS};

/// The value the dealer draws up to: it stands on all totals of seventeen or more.
pub const DEALER_STANDS: u8 = 17;
/// Default rounds dealt before the session ends on the allowance.
pub const MAX_ROUNDS: usize = 12;

/// On-screen suit symbols, indexed by [`suit_of`] (0..4).
pub const SUIT_SYMBOLS: [&str; 4] = ["♠", "♥", "♦", "♣"];

/// The rank (0..13) of a card index (0..52): ace=0, two=1, …, king=12.
pub fn rank_of(card: u8) -> u8 {
    card % RANKS as u8
}

/// The suit (0..4) of a card index (0..52), used only for display variety.
pub fn suit_of(card: u8) -> u8 {
    card / RANKS as u8
}

/// The outcome of a vingt-un session, handed to the host's `on_complete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VingtUnResult {
    /// Whether the player reached the target stake.
    pub won: bool,
    /// The stake left when the session ended (busted purse = 0).
    pub final_stake: i64,
    /// How many rounds were dealt before the session ended.
    pub rounds_played: usize,
}

/// A shuffled deck as card indices 0..52, seeded so the same `(seed, round)`
/// always deals the same cards. Each round gets its own fresh shuffle — the deal
/// is independent round to round, since the skill is the draw decision, not
/// tracking a depleting box. Fisher–Yates, the same idiom [`crate::faro`] uses.
pub fn round_deck(seed: u64, round: usize) -> Vec<u8> {
    let mut deck: Vec<u8> = (0..DECK as u8).collect();
    // Mix the round into the seed so consecutive rounds deal unrelated shuffles.
    let mut rng = GameRng::from_seed(seed ^ (round as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    shuffle(&mut deck, &mut rng);
    deck
}

/// The blackjack value of a hand and whether it is **soft** (an ace still counts
/// as eleven). Aces start at eleven and drop to one as needed to avoid busting;
/// ten, jack, queen and king all count ten.
pub fn hand_value(cards: &[u8]) -> (u8, bool) {
    let mut total: u32 = 0;
    let mut aces: u32 = 0;
    for &c in cards {
        let r = rank_of(c);
        total += match r {
            0 => {
                aces += 1;
                11
            }
            1..=8 => (r + 1) as u32, // rank 1 -> "2", … rank 8 -> "9"
            _ => 10,                 // "10", J, Q, K
        };
    }
    // Demote aces from eleven to one while the hand is over twenty-one.
    while total > 21 && aces > 0 {
        total -= 10;
        aces -= 1;
    }
    // Soft if an ace is still riding as eleven (and the hand is alive).
    let soft = aces > 0 && total <= 21;
    (total as u8, soft)
}

/// Whether a hand is over twenty-one.
pub fn is_bust(cards: &[u8]) -> bool {
    hand_value(cards).0 > 21
}

/// Whether a hand is a **vingt-et-un**: exactly two cards totalling twenty-one.
pub fn is_natural(cards: &[u8]) -> bool {
    cards.len() == 2 && hand_value(cards).0 == 21
}

/// The dealer's fixed rule: draw on any total below [`DEALER_STANDS`], stand on
/// seventeen or more (including soft seventeen).
pub fn dealer_should_hit(total: u8) -> bool {
    total < DEALER_STANDS
}

/// Play the dealer's hand out from its two `initial` cards, drawing from
/// `draw_pile` (in deal order) until it stands or the pile runs dry. Returns the
/// dealer's final hand.
pub fn play_dealer(initial: &[u8], draw_pile: &[u8]) -> Vec<u8> {
    let mut hand = initial.to_vec();
    let mut i = 0;
    while dealer_should_hit(hand_value(&hand).0) && i < draw_pile.len() {
        hand.push(draw_pile[i]);
        i += 1;
    }
    hand
}

/// Net stake change from settling a `bet` of the player's final hand against the
/// dealer's. Positive means the player gains.
///
/// A player bust always loses the stake (even if the dealer would also bust —
/// the player acted first). Naturals pay three-to-two and beat any drawn
/// twenty-one; two naturals push. Otherwise a dealer bust pays, and live hands
/// compare by total: higher wins, equal pushes.
pub fn settle(bet: i64, player: &[u8], dealer: &[u8]) -> i64 {
    let p = hand_value(player).0;
    let d = hand_value(dealer).0;
    let p_nat = is_natural(player);
    let d_nat = is_natural(dealer);

    if p > 21 {
        -bet // player busted — lost the moment they went over
    } else if p_nat && d_nat {
        0 // both naturals push
    } else if p_nat {
        bet * 3 / 2 // vingt-et-un pays a half again
    } else if d_nat {
        -bet
    } else if d > 21 {
        bet // dealer busts
    } else if p > d {
        bet
    } else if p < d {
        -bet
    } else {
        0 // equal totals push
    }
}

/// A self-contained vingt-un table. Mount it (with a `key` that changes per
/// session so the deck and stake reset) and wire `on_complete`:
///
/// ```rust,ignore
/// VingtUn {
///     key: "{night}",
///     prompt: "Under-the-Hill at Natchez. The banker deals —",
///     seed,                  // drawn from the host's RNG
///     starting_stake: 200,
///     target_stake: 500,
///     on_complete: move |r: VingtUnResult| { /* fold into the purse */ },
/// }
/// ```
#[component]
pub fn VingtUn(
    /// Line shown above the table (the situation/flavor).
    prompt: String,
    /// Seed for the per-round shuffles. Same seed → same deals (deterministic).
    seed: u64,
    /// Chips the player sits down with.
    starting_stake: i64,
    /// Stake to reach to win the session.
    target_stake: i64,
    /// Maximum rounds to deal; clamped to at least one.
    #[props(default = MAX_ROUNDS)]
    max_rounds: usize,
    /// Chips added per tap when laying a stake.
    #[props(default = 25)]
    min_bet: i64,
    /// Called once, when the session ends (target reached, bust, or rounds spent).
    on_complete: EventHandler<VingtUnResult>,
) -> Element {
    let rounds_total = max_rounds.max(1);
    // Defensive prop clamp (the kit convention, cf. faro's min_bet): a tap must
    // always be able to lay at least one chip, so cap min_bet to the opening
    // stake — otherwise min_bet > starting_stake soft-locks the table.
    let min_bet = min_bet.clamp(1, starting_stake.max(1));

    let mut stake = use_signal(move || starting_stake);
    let mut round_idx = use_signal(|| 0usize);
    // This round's wager, laid before the deal.
    let mut bet = use_signal(|| 0i64);
    // Cards dealt this round; the shared deal pointer for hits then dealer draws.
    let mut player = use_signal(Vec::<u8>::new);
    let mut dealer = use_signal(Vec::<u8>::new);
    let mut cursor = use_signal(|| 0usize);
    // Once the hand is settled, holds the dealer's revealed final hand and the
    // net delta; the round is locked.
    let mut revealed = use_signal(|| None::<(Vec<u8>, i64)>);

    // This round's shuffle. Memoized on round_idx so taps within a round don't
    // re-shuffle on every render.
    let deck = use_memo(move || round_deck(seed, round_idx()));

    let in_play = !player.read().is_empty();
    let is_revealed = revealed.read().is_some();

    // Settle the current hand against the dealer: play the dealer out (unless the
    // player already busted) and bank the delta. Shared by Stand, a player bust,
    // and a natural dealt off the top.
    let mut resolve = move || {
        let pl = player.read().clone();
        let dl_start = dealer.read().clone();
        let final_dealer = if is_bust(&pl) || is_natural(&pl) {
            // No reason for the dealer to draw: a player bust loses regardless,
            // and a natural is settled against the dealer's two cards.
            dl_start
        } else {
            let d = deck.read();
            let pile = &d[cursor().min(d.len())..];
            play_dealer(&dl_start, pile)
        };
        let delta = settle(bet(), &pl, &final_dealer);
        stake += delta;
        revealed.set(Some((final_dealer, delta)));
    };

    rsx! {
        div { class: "flex-1 flex flex-col p-3 max-w-md w-full mx-auto gap-3",
            p { class: "text-center text-sm opacity-80", "{prompt}" }

            // Status: stake / target / round.
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
                    div { class: "chip-label", "ROUND" }
                    div { class: "text-lg", "{(round_idx() + 1).min(rounds_total)}/{rounds_total}" }
                }
            }

            // The table: dealer's hand on top, player's below. Hidden once we're
            // still betting (nothing dealt yet).
            if in_play {
                {
                    let dealer_cards = match revealed() {
                        Some((final_dealer, _)) => final_dealer,
                        None => dealer.read().clone(),
                    };
                    // The hole card stays down until the hand is settled.
                    let hide_hole = !is_revealed;
                    let (dval, _) = hand_value(&dealer_cards);
                    let player_cards = player.read().clone();
                    let (pval, psoft) = hand_value(&player_cards);
                    rsx! {
                        Hand {
                            label: "DEALER",
                            cards: dealer_cards.clone(),
                            hide_from: if hide_hole { Some(1usize) } else { None },
                            total: if hide_hole { None } else { Some(dval) },
                            soft: false,
                            round: round_idx(),
                        }
                        Hand {
                            label: "YOU",
                            cards: player_cards,
                            hide_from: None,
                            total: Some(pval),
                            soft: psoft,
                            round: round_idx(),
                        }
                    }
                }
            } else {
                p { class: "text-center text-xs opacity-60",
                    "Lay your stake "
                    "{fmt_dollars_compact(min_bet as f64)}"
                    " at a tap, then deal. Draw toward 21 — a two-card 21 pays 3:2."
                }
            }

            // Outcome line, once the hand is settled.
            if let Some((final_dealer, delta)) = revealed() {
                {
                    let pl = player.read().clone();
                    rsx! {
                        div { class: "text-center text-sm",
                            if is_bust(&pl) {
                                span { class: "chip-danger", "Bust — over 21. " }
                            } else if is_natural(&pl) {
                                span { "Vingt-et-un! " }
                            } else if is_bust(&final_dealer) {
                                span { "Dealer busts. " }
                            }
                            match delta {
                                d if d > 0 => rsx! { span { "You win {fmt_dollars_compact(d as f64)}." } },
                                d if d < 0 => rsx! { span { class: "chip-danger", "You lose {fmt_dollars_compact((-d) as f64)}." } },
                                _ => rsx! { span { class: "opacity-70", "Push — stake returned." } },
                            }
                        }
                    }
                }
            }

            // Controls.
            if is_revealed {
                {
                    let played = round_idx() + 1;
                    let over = session_over(stake(), target_stake, played, rounds_total);
                    // A win means reaching the target while still solvent — a busted
                    // purse never counts, even if a host set a non-positive target.
                    let won = stake() > 0 && stake() >= target_stake;
                    rsx! {
                        button {
                            class: "{BTN_PRIMARY}",
                            onclick: move |_| {
                                if over {
                                    on_complete.call(VingtUnResult {
                                        won,
                                        final_stake: stake().max(0),
                                        rounds_played: played,
                                    });
                                } else {
                                    round_idx += 1;
                                    bet.set(0);
                                    player.set(Vec::new());
                                    dealer.set(Vec::new());
                                    cursor.set(0);
                                    revealed.set(None);
                                }
                            },
                            if over {
                                if won { "Cash out — you beat the bank" }
                                else if stake() <= 0 { "Tapped out — leave the table" }
                                else { "The night is over — leave the table" }
                            } else { "Next round" }
                        }
                    }
                }
            } else if in_play {
                // Player's turn: draw or stand.
                div { class: "grid grid-cols-2 gap-2",
                    button {
                        class: "{BTN}",
                        onclick: move |_| {
                            if is_revealed { return; }
                            let d = deck.read();
                            let c = cursor();
                            if c >= d.len() { return; }
                            let mut hand = player.read().clone();
                            hand.push(d[c]);
                            player.set(hand);
                            cursor += 1;
                            if is_bust(&player.read()) {
                                resolve();
                            }
                        },
                        "Draw"
                    }
                    button {
                        class: "{BTN}",
                        onclick: move |_| {
                            if is_revealed { return; }
                            resolve();
                        },
                        "Stand"
                    }
                }
            } else {
                // Betting: lay the stake, then deal.
                div { class: "grid grid-cols-2 gap-2",
                    button {
                        class: "{BTN}",
                        onclick: move |_| {
                            let next = bet() + min_bet;
                            if next <= stake() {
                                bet.set(next);
                            }
                        },
                        "Stake +{fmt_dollars_compact(min_bet as f64)}"
                    }
                    button {
                        class: "{BTN}",
                        disabled: bet() == 0,
                        onclick: move |_| bet.set(0),
                        "Clear"
                    }
                }
                button {
                    class: "{BTN_PRIMARY}",
                    disabled: bet() == 0 || bet() > stake(),
                    onclick: move |_| {
                        if in_play { return; }
                        let d = deck.read();
                        // Standard alternating deal: player, dealer, player, dealer.
                        player.set(vec![d[0], d[2]]);
                        dealer.set(vec![d[1], d[3]]);
                        cursor.set(4);
                        drop(d);
                        // A natural on either side settles the hand on the spot.
                        if is_natural(&player.read()) || is_natural(&dealer.read()) {
                            resolve();
                        }
                    },
                    if bet() > 0 { "Deal — {fmt_dollars_compact(bet() as f64)} at stake" } else { "Lay a stake to deal" }
                }
            }
        }
    }
}

/// One side's hand: a label, a row of cards (some optionally face-down via
/// `hide_from`), and the running total when it should be shown. Each card flips
/// in on mount; new cards as a hand grows replay the flip because their `key`
/// (round + position) is fresh.
#[component]
fn Hand(
    label: String,
    cards: Vec<u8>,
    /// Cards at this index and beyond are dealt face-down (the dealer's hole).
    hide_from: Option<usize>,
    /// The total to show, or `None` to keep it hidden (hole card still down).
    total: Option<u8>,
    soft: bool,
    /// The round number, mixed into card keys so a fresh deal replays the flip.
    round: usize,
) -> Element {
    let busted = total.is_some_and(|t| t > 21);
    rsx! {
        div { class: "flex flex-col items-center gap-1",
            div { class: "flex items-center gap-2",
                span { class: "chip-label", "{label}" }
                if let Some(t) = total {
                    span {
                        class: if busted { "chip-danger text-sm" } else { "text-sm" },
                        if busted { "BUST" } else { "{t}" }
                        if soft && !busted { span { class: "opacity-60", " soft" } }
                    }
                }
            }
            div { class: "flex items-stretch justify-center gap-2",
                for (i, &c) in cards.iter().enumerate() {
                    Card {
                        key: "{round}-{i}-{c}",
                        card: c,
                        face_down: hide_from.is_some_and(|h| i >= h),
                    }
                }
            }
        }
    }
}

/// One card, dealt in with the shared [`FlipCard`] flip (self-contained, so no
/// global CSS leaks). A face-down card shows a phosphor back instead of its rank.
#[component]
fn Card(card: u8, face_down: bool) -> Element {
    let r = rank_of(card) as usize;
    let s = suit_of(card) as usize;
    let red = s == 1 || s == 2; // hearts and diamonds
    rsx! {
        FlipCard { width: "3rem",
            if face_down {
                div { class: "text-2xl leading-none opacity-50", "🂠" }
            } else {
                div {
                    class: if red { "text-xl leading-none chip-danger" } else { "text-xl leading-none" },
                    "{RANK_LABELS[r]}"
                    span { class: "opacity-70", "{SUIT_SYMBOLS[s]}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a card index from a rank (0..13) and suit (0..4).
    fn card(rank: u8, suit: u8) -> u8 {
        suit * RANKS as u8 + rank
    }

    #[test]
    fn deck_is_a_full_52_unique_cards() {
        let deck = round_deck(12345, 0);
        assert_eq!(deck.len(), DECK);
        let mut seen = [false; DECK];
        for &c in &deck {
            assert!((c as usize) < DECK);
            assert!(!seen[c as usize], "card {c} appeared twice");
            seen[c as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn deck_is_deterministic_per_seed_and_round() {
        assert_eq!(round_deck(7, 3), round_deck(7, 3));
        // Different rounds of the same seed deal different shuffles.
        assert!((0..32).any(|r| round_deck(7, r) != round_deck(7, 0)));
        // Different seeds deal different shuffles.
        assert!((0..32).any(|s| round_deck(s, 0) != round_deck(7, 0)));
    }

    #[test]
    fn rank_and_suit_decompose_the_index() {
        for c in 0..DECK as u8 {
            assert!((rank_of(c) as usize) < RANKS);
            assert!((suit_of(c) as usize) < 4);
            assert_eq!(suit_of(c) * RANKS as u8 + rank_of(c), c);
        }
    }

    #[test]
    fn hand_value_counts_pips_faces_and_hard_aces() {
        // 9 + 10(king) = 19, no ace -> hard.
        assert_eq!(hand_value(&[card(8, 0), card(12, 1)]), (19, false));
        // 10 + 7 = 17 hard.
        assert_eq!(hand_value(&[card(9, 0), card(6, 0)]), (17, false));
        // Two as pips: 2 + 5 = 7.
        assert_eq!(hand_value(&[card(1, 0), card(4, 0)]), (7, false));
    }

    #[test]
    fn ace_is_soft_then_demotes_to_avoid_bust() {
        // A + 6 = 17 soft.
        assert_eq!(hand_value(&[card(0, 0), card(5, 0)]), (17, true));
        // A + 6 + 10 = 17 hard (ace demoted to 1).
        assert_eq!(hand_value(&[card(0, 0), card(5, 0), card(9, 0)]), (17, false));
        // A + A = 12 soft (one ace 11, one ace 1).
        assert_eq!(hand_value(&[card(0, 0), card(0, 1)]), (12, true));
        // A + A + 9 = 21 soft.
        assert_eq!(hand_value(&[card(0, 0), card(0, 1), card(8, 0)]), (21, true));
    }

    #[test]
    fn natural_and_bust_detection() {
        // A + king = 21 in two cards -> natural.
        assert!(is_natural(&[card(0, 0), card(12, 0)]));
        // 21 in three cards is not a natural.
        assert!(!is_natural(&[card(6, 0), card(6, 1), card(6, 2)]));
        assert!(!is_natural(&[card(0, 0)]));
        // Over 21 busts.
        assert!(is_bust(&[card(12, 0), card(12, 1), card(4, 0)])); // 10+10+5
        assert!(!is_bust(&[card(12, 0), card(12, 1)])); // 20
    }

    #[test]
    fn dealer_draws_to_seventeen_and_stands() {
        assert!(dealer_should_hit(16));
        assert!(!dealer_should_hit(17));
        assert!(!dealer_should_hit(21));
        // Starts 10 + 5 = 15, draws a 6 -> 21, stands.
        let final_hand = play_dealer(&[card(9, 0), card(4, 0)], &[card(5, 1)]);
        assert_eq!(hand_value(&final_hand).0, 21);
        assert_eq!(final_hand.len(), 3);
        // Starts on 18, never draws.
        let stood = play_dealer(&[card(9, 0), card(7, 0)], &[card(9, 1)]);
        assert_eq!(stood.len(), 2);
    }

    #[test]
    fn dealer_stops_when_pile_runs_dry() {
        // Starts on 12, pile empty -> can't draw, stands short.
        let hand = play_dealer(&[card(1, 0), card(9, 0)], &[]);
        assert_eq!(hand.len(), 2);
        assert_eq!(hand_value(&hand).0, 12);
    }

    #[test]
    fn settle_player_bust_always_loses() {
        let player = [card(12, 0), card(12, 1), card(4, 0)]; // 25, bust
        let dealer_bust = [card(12, 2), card(12, 3), card(5, 0)]; // 25, also bust
        assert_eq!(settle(100, &player, &dealer_bust), -100);
    }

    #[test]
    fn settle_naturals_pay_three_to_two_and_push() {
        let nat = [card(0, 0), card(12, 0)]; // A + K = 21 natural
        let twenty = [card(12, 1), card(9, 0)]; // 20
        assert_eq!(settle(100, &nat, &twenty), 150); // 3:2
        let other_nat = [card(0, 1), card(12, 2)];
        assert_eq!(settle(100, &nat, &other_nat), 0); // push
        // Dealer natural beats the player's plain 21.
        let drawn_21 = [card(6, 0), card(6, 1), card(6, 2)]; // 21 in three
        assert_eq!(settle(100, &drawn_21, &other_nat), -100);
    }

    #[test]
    fn settle_compares_totals_and_dealer_bust() {
        let p20 = [card(12, 0), card(9, 0)]; // 20
        let d18 = [card(9, 1), card(7, 0)]; // 18
        assert_eq!(settle(100, &p20, &d18), 100); // higher wins
        assert_eq!(settle(100, &d18, &p20), -100); // lower loses
        assert_eq!(settle(100, &p20, &p20), 0); // equal pushes
        let d_bust = [card(12, 2), card(12, 3), card(4, 0)]; // 24
        assert_eq!(settle(100, &p20, &d_bust), 100); // dealer bust pays
    }

    #[test]
    fn settle_rounds_natural_payout_down() {
        let nat = [card(0, 0), card(12, 0)];
        let twenty = [card(12, 1), card(9, 0)];
        assert_eq!(settle(101, &nat, &twenty), 151); // 101*3/2 = 151
    }

    #[test]
    fn session_over_on_bust_target_or_allowance() {
        assert!(session_over(0, 500, 1, 12)); // busted purse
        assert!(session_over(-50, 500, 1, 12));
        assert!(session_over(500, 500, 1, 12)); // hit target
        assert!(session_over(600, 500, 1, 12));
        assert!(session_over(300, 500, 12, 12)); // rounds spent
        assert!(!session_over(300, 500, 5, 12)); // still playing
    }
}
