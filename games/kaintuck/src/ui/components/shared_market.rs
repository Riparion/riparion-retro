//! Multiplayer surface: a slim bar showing the shared-market connection state.
//! Renders nothing in offline single-player (the `RemoteMarket` context signal
//! stays `None` when no server is configured), so the offline game looks exactly
//! as it always has. (Trader gossip is voiced through the engine's normal travel
//! banter — see `kaintuck_engine`'s gossip feed — not here.)

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

    rsx! {
        section { class: "status-bar shrink-0 px-3 py-1 text-xs",
            div { class: "flex items-center gap-2",
                span { style: "color: {dot_color}; text-shadow: var(--glow);", "{dot}" }
                span { class: "tracking-widest opacity-80", "{label}" }
            }
        }
    }
}
