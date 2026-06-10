//! The classic command line: a text box that raises the mobile soft keyboard,
//! a horizontally-scrollable autocomplete chip strip drawn from the game's
//! vocabulary, and a GO button. Enter submits too.

use dioxus::prelude::*;

use crate::engine::autocomplete;

#[component]
pub fn CommandInput(on_submit: EventHandler<String>) -> Element {
    let mut text = use_signal(String::new);

    // Suggestions for the word currently being typed (the last token).
    let suggestions = use_memo(move || {
        let t = text();
        let last = t.split_whitespace().last().unwrap_or("");
        let prefix = if t.ends_with(char::is_whitespace) { "" } else { last };
        autocomplete(prefix, 8)
    });

    rsx! {
        div { class: "cmd-dock shrink-0",
            div { class: "suggestions flex gap-2 overflow-x-auto px-2 py-1",
                for w in suggestions() {
                    {
                        let chip = w.clone();
                        rsx! {
                            button {
                                key: "{w}",
                                class: "chip-btn",
                                onclick: move |_| {
                                    let cur = text();
                                    let mut base = match cur.rfind(char::is_whitespace) {
                                        Some(pos) => cur[..=pos].to_string(),
                                        None => String::new(),
                                    };
                                    base.push_str(&chip);
                                    base.push(' ');
                                    text.set(base);
                                },
                                "{w}"
                            }
                        }
                    }
                }
            }
            div { class: "input-row flex gap-2 p-2",
                input {
                    class: "crt-input flex-1",
                    r#type: "text",
                    value: "{text}",
                    autocapitalize: "off",
                    autocomplete: "off",
                    autocorrect: "off",
                    spellcheck: "false",
                    placeholder: "type a command…",
                    oninput: move |e| text.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            let cmd = text().trim().to_string();
                            if !cmd.is_empty() {
                                on_submit.call(cmd);
                                text.set(String::new());
                            }
                        }
                    },
                }
                button {
                    class: "crt-btn crt-btn-primary go-btn",
                    onclick: move |_| {
                        let cmd = text().trim().to_string();
                        if !cmd.is_empty() {
                            on_submit.call(cmd);
                            text.set(String::new());
                        }
                    },
                    "GO"
                }
            }
        }
    }
}
