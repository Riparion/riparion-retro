//! Touch-friendly quantity/amount entry: numeric soft keyboard, live cost
//! preview, and an "All" button mirroring the original's `A` shortcut.

use dioxus::prelude::*;

use crate::format::fmt_money;
use crate::theme::{BTN, BTN_PRIMARY};

#[component]
pub fn NumberEntry(
    /// Reactive so a new question never keeps the old answer: the entered
    /// value clears whenever the prompt changes.
    prompt: ReadSignal<String>,
    /// Upper bound; "All" fills this in and submission clamps to it.
    max: i64,
    /// Unit price for a live cost/proceeds preview.
    #[props(default)] price: Option<f64>,
    /// Custom live preview of the entered amount ("feeds 95 people", ...).
    /// Takes precedence over `price` when both are given.
    #[props(default)] preview: Option<Callback<i64, String>>,
    #[props(default = "Confirm".to_string())] confirm: String,
    on_submit: EventHandler<i64>,
    /// Omit to hide the Cancel button (e.g. when there is no screen to
    /// cancel back to).
    #[props(default)] on_cancel: Option<EventHandler<()>>,
) -> Element {
    let mut value = use_signal(String::new);
    use_effect(move || {
        prompt.read(); // subscribe: a different question voids the answer
        value.set(String::new());
    });
    let parsed = use_memo(move || value.read().parse::<i64>().unwrap_or(0).clamp(0, i64::MAX));
    let amount = parsed().min(max);
    let preview_text = match (&preview, price) {
        (Some(f), _) => Some(f.call(amount)),
        (None, Some(p)) => Some(format!("= {}", fmt_money(p * amount as f64))),
        (None, None) => None,
    };

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
            if let Some(text) = preview_text {
                p { class: "text-sm opacity-80", "{text}" }
            }
            div { class: "flex gap-2",
                if let Some(on_cancel) = on_cancel {
                    button {
                        class: "{BTN} flex-1",
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
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
