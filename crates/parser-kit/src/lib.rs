//! Forgiving-input inference for limited-vocabulary text parsers (text
//! adventures, verb/noun command games, …) — entirely offline, no LLM.
//!
//! A classic parser resolves each typed word against a fixed vocabulary by exact
//! match, so `take the lamp`, `pick up lamp`, and a typo like `lmap` all fail.
//! This crate recovers intent from such near-misses with a tiered pipeline:
//! filler-stripping → phrasal-verb collapse → synonym/alias expansion →
//! collapse-to-pair → fuzzy typo correction → "did you mean?". See [`engine::infer`].
//!
//! ## Caller owns content and policy
//!
//! The crate is deliberately ignorant of any specific game. The consumer supplies:
//! - the **tables** ([`InferConfig`]): filler words, aliases, phrasal verbs, thresholds;
//! - the **candidate set**: full-spelling vocabulary for fuzzy matching;
//! - a **`resolves_ok`** closure: "would these tokens parse as a command today?" —
//!   this is both the gate that keeps already-valid input untouched and the
//!   success test after each tier;
//! - a **`present`** closure: "does this word name something in scope?" — used to
//!   rank fuzzy candidates (prefer what the player can actually see).
//!
//! Because every transformation is gated behind `resolves_ok`, input the caller
//! already understands is returned [`Inference::Unchanged`] without modification —
//! the inference layer is provably inert on the happy path.

pub mod config;
pub mod distance;
pub mod engine;
pub mod resolve;

pub use config::{InferConfig, LenThreshold};
pub use distance::damerau_levenshtein_bounded;
pub use engine::{infer, Inference};
pub use resolve::{best_match, Match};
