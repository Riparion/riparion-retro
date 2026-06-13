//! Optional online leaderboards for games served by riparion-cms.
//!
//! A game served as a published bundle finds the CMS API through the
//! `<meta name="riparion-api-base">` tag the server injects into the bundle's
//! HTML at serve time. That base is the CMS's *canonical* origin (the API host),
//! not the apps origin the bundle runs on — the two differ by design, so a
//! relative URL wouldn't reach the API. Because the meta is injected per request
//! by whichever CMS process serves the bundle, the same promoted bytes target QA
//! on the QA server and prod on the prod server; the URL is never baked in.
//!
//! When the meta is absent — plain `dx serve`, a non-CMS host, or a non-browser
//! build — every call here degrades to a no-op (`submit` succeeds silently,
//! `fetch_top` returns an empty board), so a game stays fully playable offline
//! with its local high-score table.
//!
//! Identity is anonymous by design: a submission carries a free-text handle and
//! an opaque per-run JSON payload, and the server throttles by a coarse client
//! hash. The CMS deliberately keeps its session cookie off the apps origin, so
//! there is no logged-in identity to assert from here.

use serde::Serialize;

/// The `<meta name>` the CMS injects with the API base.
const META_NAME: &str = "riparion-api-base";

/// The injected CMS API base (scheme + host, no trailing slash), or `None` when
/// the game isn't served by a CMS — the signal to run local-only. Reads the
/// `<meta name="riparion-api-base">` content from the document; absent outside a
/// browser or under `dx serve`.
pub fn api_base() -> Option<String> {
    let doc = web_sys::window()?.document()?;
    let el = doc
        .query_selector(&format!("meta[name=\"{META_NAME}\"]"))
        .ok()??;
    let content = el.get_attribute("content")?;
    let trimmed = content.trim().trim_end_matches('/');
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Whether online leaderboards are available (a CMS injected an API base). Cheap
/// enough to call from a component body to decide whether to show an online
/// board.
pub fn enabled() -> bool {
    api_base().is_some()
}

/// One row from a board read. `payload` is the game's own per-run JSON echoed
/// back unparsed (rank label, win flag, day count, …) — `Null` when the row
/// carried none. The game interprets it; the kit stays game-agnostic.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Entry {
    pub rank: i64,
    pub handle: String,
    pub score: i64,
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// The endpoint for an app's board.
fn scores_url(base: &str, slug: &str) -> String {
    format!("{base}/api/apps/{slug}/scores")
}

/// Submit one finished run to `slug`'s `board`. A no-op returning `Ok(())` when
/// the game isn't served by a CMS, so callers can fire-and-forget unconditionally.
/// `payload` is any `Serialize` value to echo back on reads (run stats, version).
///
/// Sent as a `text/plain` body so it stays a CORS "simple request" (no preflight
/// round-trip); the CMS parses the JSON regardless of content type.
pub async fn submit<P: Serialize>(
    slug: &str,
    board: &str,
    handle: &str,
    score: i64,
    payload: &P,
) -> Result<(), String> {
    let Some(base) = api_base() else {
        return Ok(());
    };
    let body = serde_json::json!({
        "board": board,
        "handle": handle,
        "score": score,
        "payload": payload,
    })
    .to_string();
    let resp = gloo_net::http::Request::post(&scores_url(&base, slug))
        .header("Content-Type", "text/plain;charset=UTF-8")
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(format!("leaderboard submit failed: HTTP {}", resp.status()))
    }
}

/// Fetch the top `limit` scores for `slug`'s `board`, highest first. Returns an
/// empty vec (not an error) when the game isn't served by a CMS, so "offline"
/// and "empty board" render the same.
pub async fn fetch_top(slug: &str, board: &str, limit: u32) -> Result<Vec<Entry>, String> {
    let Some(base) = api_base() else {
        return Ok(Vec::new());
    };
    let url = format!("{}?board={board}&limit={limit}", scores_url(&base, slug));
    let resp = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("leaderboard fetch failed: HTTP {}", resp.status()));
    }
    resp.json::<Vec<Entry>>().await.map_err(|e| e.to_string())
}
