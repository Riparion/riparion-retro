//! A sheet: a drawer that slides in over a dimming scrim, holding actions or
//! content a screen would otherwise stack inline. Slides up from the bottom edge
//! by default; pass `from_top` for a notification-center style panel that drops
//! down from the top. The parent owns the open/closed `Signal<bool>` and passes
//! it in; tapping the scrim (or any action that calls `on_close`) dismisses it.

use dioxus::prelude::*;

#[component]
pub fn Sheet(
    open: bool,
    on_close: EventHandler<()>,
    /// Optional caption shown across the top of the sheet.
    #[props(default)]
    title: Option<String>,
    /// Slide down from the top edge instead of up from the bottom.
    #[props(default)]
    from_top: bool,
    children: Element,
) -> Element {
    // Both states stay in the DOM so the CSS transition runs on toggle; only the
    // `open` class flips to drive the slide and scrim fade. The `-top` modifier
    // swaps the anchor edge and slide direction.
    let base = if from_top { "crt-sheet crt-sheet-top" } else { "crt-sheet" };
    let panel = if open { format!("{base} open") } else { base.to_string() };
    let scrim = if open { "crt-sheet-scrim open" } else { "crt-sheet-scrim" };
    rsx! {
        div { class: "{scrim}", onclick: move |_| on_close.call(()) }
        div { class: "{panel}",
            // The grip sits at the panel's leading edge for both variants; it stays
            // pinned there rather than scrolling away with a long body.
            div { class: "crt-sheet-grip" }
            if let Some(t) = title {
                div { class: "chip-label mb-2 text-center", "{t}" }
            }
            {children}
        }
    }
}
