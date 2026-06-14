//! A bottom sheet: a drawer that slides up from the bottom edge over a dimming
//! scrim, holding the secondary actions a screen would otherwise stack into its
//! action bar. The parent owns the open/closed `Signal<bool>` and passes it in;
//! tapping the scrim (or any action that calls `on_close`) dismisses it.

use dioxus::prelude::*;

#[component]
pub fn Sheet(
    open: bool,
    on_close: EventHandler<()>,
    /// Optional caption shown across the top of the sheet.
    #[props(default)]
    title: Option<String>,
    children: Element,
) -> Element {
    // Both states stay in the DOM so the CSS transition runs on toggle; only the
    // `open` class flips to drive the slide-up and scrim fade.
    let panel = if open { "crt-sheet open" } else { "crt-sheet" };
    let scrim = if open { "crt-sheet-scrim open" } else { "crt-sheet-scrim" };
    rsx! {
        div { class: "{scrim}", onclick: move |_| on_close.call(()) }
        div { class: "{panel}",
            div { class: "crt-sheet-grip" }
            if let Some(t) = title {
                div { class: "chip-label mb-2 text-center", "{t}" }
            }
            {children}
        }
    }
}
