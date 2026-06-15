//! Multiplayer surface: a slim bar showing the shared-market connection state
//! and a live feed of "news from the river" — composed remarks about the other
//! traders the server reports. Renders nothing in offline single-player (the
//! `RemoteMarket` context signal stays `None` when no server is configured), so
//! the offline game looks exactly as it always has.

use dioxus::prelude::*;

use crate::net_client::RemoteMarket;

#[component]
pub fn SharedMarket() -> Element {
    let remote = use_context::<Signal<Option<RemoteMarket>>>();
    let guard = remote.read();
    // Offline (no server / never connected) → render nothing at all.
    let Some(rm) = guard.as_ref() else {
        return rsx! {};
    };

    let (dot, dot_color, label) = if rm.connected {
        ("●", "var(--phosphor)", "shared market")
    } else {
        ("○", "var(--danger)", "reconnecting…")
    };

    // Newest first, capped for the bar; the buffer itself is already bounded.
    let lines: Vec<String> = rm.gossip_log.iter().rev().take(4).cloned().collect();

    rsx! {
        section { class: "status-bar shrink-0 px-3 py-1 text-xs",
            div { class: "flex items-center gap-2",
                span { style: "color: {dot_color}; text-shadow: var(--glow);", "{dot}" }
                span { class: "tracking-widest opacity-80", "{label}" }
            }
            if lines.is_empty() {
                div { class: "opacity-50 truncate", style: "font-style: italic;", "listening for news from the river…" }
            } else {
                ul { class: "mt-1",
                    for line in lines {
                        li { class: "opacity-70 truncate", "“{line}”" }
                    }
                }
            }
        }
    }
}
