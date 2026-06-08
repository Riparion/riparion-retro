//! One labeled money-allocation row: a name, an optional hint, a `$` numeric
//! input, and an optional derived note (e.g. "= 3,000 bullets"). The parent
//! owns the amounts and the running total — this is just the repeated row,
//! shared by outfitting and shop screens where the player splits a budget
//! across several categories at once. (For a single quantity/amount with an
//! "All" button, reach for [`super::number_entry::NumberEntry`] instead.)

use dioxus::prelude::*;

use crate::theme::PANEL;

#[component]
pub fn SpendRow(
    label: String,
    /// Secondary descriptive line under the label (empty hides it).
    #[props(default)] hint: String,
    /// A derived line that updates as the amount changes (what you get, etc.).
    #[props(default)] note: Option<String>,
    /// Current dollar amount.
    value: i64,
    /// Called with the parsed, non-negative amount on each edit.
    on_input: EventHandler<i64>,
) -> Element {
    rsx! {
        div { class: "{PANEL} p-2 flex items-center gap-3",
            div { class: "flex-1",
                div { "{label}" }
                if !hint.is_empty() {
                    div { class: "text-xs opacity-60", "{hint}" }
                }
                if let Some(note) = note {
                    div { class: "text-xs opacity-80", "{note}" }
                }
            }
            div { class: "flex items-center gap-1",
                span { class: "opacity-60", "$" }
                input {
                    class: "crt-input w-20 text-right",
                    r#type: "number",
                    inputmode: "numeric",
                    min: "0",
                    value: "{value}",
                    oninput: move |e: FormEvent| {
                        on_input.call(e.value().parse::<i64>().unwrap_or(0).max(0));
                    },
                }
            }
        }
    }
}
