//! Shared kit for the retro games in this workspace: the green-phosphor CRT
//! aesthetic, touch-first UI components, and platform primitives (seedable
//! RNG, money formatting, versioned localStorage persistence).
//!
//! Game crates link the stylesheet once and build on the pieces:
//!
//! ```rust,ignore
//! document::Stylesheet { href: retro_kit::CRT_CSS }
//! ```

pub mod components;
pub mod game_flow;
pub mod leaderboard;
pub mod playback;
pub mod rations;
pub mod scoring;
pub mod storage;
pub mod theme;

// The seedable RNG and money/number formatting now live in the pure
// `retro-core` crate (no Dioxus/gloo/web-sys), so headless engines and the
// native server can share them. Re-exported here so existing
// `retro_kit::rng` / `retro_kit::format` paths keep working unchanged.
pub use retro_core::{format, rng};

use dioxus::prelude::*;

/// The shared CRT stylesheet: phosphor palette, scanlines, buttons, inputs,
/// panels, chips, action bar. Game-specific flourishes belong in each game's
/// own css, layered on top of this.
pub const CRT_CSS: Asset = asset!("/assets/crt.css");

/// Read a `<meta name="…">` tag's trimmed `content` from the document. `None`
/// outside a browser or when the tag is absent or empty. Used to pick up
/// server-injected configuration (the leaderboard API base, the multiplayer
/// WebSocket endpoint) without baking a URL into the bundle.
pub fn meta_content(name: &str) -> Option<String> {
    let doc = web_sys::window()?.document()?;
    let el = doc
        .query_selector(&format!("meta[name=\"{name}\"]"))
        .ok()??;
    let content = el.get_attribute("content")?;
    let trimmed = content.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Read a `?name=value` query parameter from the current URL. `None` outside a
/// browser or when the param is absent. Unlike [`meta_content`] (server-injected
/// config) this is a developer-typed switch: use it for opt-in dev shortcuts
/// that must never be baked into the bundle — e.g. `?dev_start=natchez`. Values
/// are returned raw (not percent-decoded), which is fine for simple flag words.
pub fn query_param(name: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    let query = search.strip_prefix('?').unwrap_or(&search);
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == name).then(|| v.to_string())
    })
}
