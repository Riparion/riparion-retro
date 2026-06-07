#!/usr/bin/env bash
# Scaffold a new game crate in this workspace.
#
#   scripts/new-game.sh <name>
#
# Creates games/<name>/ with the conventions from NOTES.md: a workspace-
# inheriting Cargo.toml, Dioxus + Tailwind config, and an app shell already
# wired to retro-kit's CRT theme. The workspace member glob picks it up
# automatically — after this, just `cd games/<name> && dx serve`.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
name="${1:-}"

if [[ -z "$name" ]]; then
    echo "usage: scripts/new-game.sh <name>" >&2
    exit 1
fi
if [[ ! "$name" =~ ^[a-z][a-z0-9-]*$ ]]; then
    echo "error: name must be lowercase kebab-case ([a-z][a-z0-9-]*), got '$name'" >&2
    exit 1
fi
dir="$root/games/$name"
if [[ -e "$dir" ]]; then
    echo "error: $dir already exists" >&2
    exit 1
fi

# "dead-mans-hand" -> "DEAD MANS HAND" for the splash title.
title="$(echo "$name" | tr 'a-z-' 'A-Z ')"

mkdir -p "$dir/src" "$dir/assets"

cat > "$dir/Cargo.toml" <<EOF
[package]
name = "$name"
version = "0.1.0"
authors.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
retro-kit = { workspace = true }

dioxus = { workspace = true }
serde = { workspace = true }

[features]
default = ["web"]
web = ["dioxus/web"]
EOF

cat > "$dir/Dioxus.toml" <<EOF
[application]

[web.app]
title = "$name"
EOF

# Tailwind: dx auto-compiles the root input into assets/tailwind.css. The
# @source pulls in utility classes used inside retro-kit's shared components
# (the scan only covers the game crate's own sources by default).
printf '@import "tailwindcss";\n@source "../../crates/retro-kit/src";\n' > "$dir/tailwind.css"
: > "$dir/assets/tailwind.css"

cat > "$dir/assets/main.css" <<EOF
/* $name — game-specific styling. The CRT identity (palette, scanlines,
   buttons, inputs, panels) comes from retro-kit's crt.css; layer on the
   shared CSS vars (--phosphor, --phosphor-dim, ...) — don't fork them. */
EOF

cat > "$dir/src/main.rs" <<EOF
mod app;

fn main() {
    dioxus::launch(app::App);
}
EOF

cat > "$dir/src/app.rs" <<EOF
use dioxus::prelude::*;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: MAIN_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1, viewport-fit=cover, user-scalable=no",
        }
        document::Title { "$name" }
        div { class: "crt flex flex-col h-[100dvh] items-center justify-center gap-6 p-6 text-center",
            h1 { class: "splash-title", "$title" }
            p { class: "opacity-80 max-w-xs mx-auto leading-snug",
                "A new game. See NOTES.md at the workspace root for how to build it."
            }
            button { class: "crt-btn crt-btn-primary max-w-xs w-full text-xl", "START" }
        }
    }
}
EOF

echo "Created games/$name/"
echo
echo "Next steps:"
echo "  cd games/$name && dx serve     # renders the CRT splash immediately"
echo "  cargo test                     # from the root once you add an engine"
echo
echo "Read NOTES.md for the architecture pattern (pure engine + Signal<Game>"
echo "+ Mode enum + interaction queue) and the verification checklist."
