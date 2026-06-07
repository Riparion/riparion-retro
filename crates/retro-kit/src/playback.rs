//! Paced playback of a turn's messages: the engine resolves synchronously,
//! the UI reveals the result line by line. Await this inside a `spawn`ed
//! task; the finish logic (extra delays, mode flips, busy flags) stays with
//! the caller — that part genuinely differs per game.

use gloo_timers::future::TimeoutFuture;

/// Feed `items` to `on_item` with `item_ms` between reveals. With
/// `trailing_delay` the pause also follows the last item (taipan's combat
/// beat); without it the last item lands and control returns immediately.
pub async fn play_paced<T>(
    items: Vec<T>,
    item_ms: u32,
    trailing_delay: bool,
    mut on_item: impl FnMut(T),
) {
    let count = items.len();
    for (idx, item) in items.into_iter().enumerate() {
        on_item(item);
        if trailing_delay || idx + 1 < count {
            TimeoutFuture::new(item_ms).await;
        }
    }
}
