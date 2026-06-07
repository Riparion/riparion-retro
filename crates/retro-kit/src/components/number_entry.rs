//! Touch-friendly quantity/amount entry: numeric soft keyboard, live cost
//! preview, and an "All" button mirroring the original's `A` shortcut.

use dioxus::prelude::*;

use crate::format::fmt_money;
use crate::theme::{BTN, BTN_PRIMARY};

#[component]
pub fn NumberEntry(
    prompt: String,
    /// Upper bound; "All" fills this in and submission clamps to it.
    max: i64,
    /// Unit price for a live cost/proceeds preview.
    #[props(default)] price: Option<f64>,
    #[props(default = "Confirm".to_string())] confirm: String,
    on_submit: EventHandler<i64>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut value = use_signal(String::new);
    let parsed = use_memo(move || value.read().parse::<i64>().unwrap_or(0).clamp(0, i64::MAX));
    let amount = parsed().min(max);
    let cost = price.map(|p| p * amount as f64);

    rsx! {
        div { class: "crt-panel m-3 p-3 flex flex-col gap-3",
            p { class: "leading-snug", "{prompt}" }
            div { class: "flex items-center gap-2",
                input {
                    class: "crt-input flex-1",
                    r#type: "number",
                    inputmode: "numeric",
                    pattern: "[0-9]*",
                    min: "0",
                    placeholder: "0",
                    value: "{value}",
                    autofocus: true,
                    oninput: move |e| value.set(e.value()),
                }
                button {
                    class: "{BTN}",
                    onclick: move |_| value.set(max.to_string()),
                    "All ({max})"
                }
            }
            if let Some(cost) = cost {
                p { class: "text-sm opacity-80", "= {fmt_money(cost)}" }
            }
            div { class: "flex gap-2",
                button {
                    class: "{BTN} flex-1",
                    onclick: move |_| on_cancel.call(()),
                    "Cancel"
                }
                button {
                    class: "{BTN_PRIMARY} flex-1",
                    onclick: move |_| on_submit.call(amount),
                    "{confirm}"
                }
            }
        }
    }
}
