//! The classic command line: a text box that raises the mobile soft keyboard
//! and a GO button. Enter submits too. The parser infers intent from forgiving
//! input (see `engine::parse_infer`), so there's no vocabulary chip strip.

use dioxus::prelude::*;

#[component]
pub fn CommandInput(on_submit: EventHandler<String>) -> Element {
    let mut text = use_signal(String::new);

    rsx! {
        div { class: "cmd-dock shrink-0",
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
