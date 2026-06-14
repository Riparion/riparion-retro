# Riparion Retro — dev tasks. Run `just` to list recipes.
#
# Every serve/build recipe passes --debug-symbols=false: dx 0.7.9's pinned
# binaryen (v127) SIGABRTs running wasm-opt over the DWARF 5 that rustc emits
# ("compile unit size was incorrect / unsupported version of DWARF"). dx then
# silently ships the UN-optimized wasm, so the app still launches but every run
# spews the abort. Dropping debug symbols (you'd need a browser extension to use
# them anyway) lets wasm-opt run clean.

# List available recipes
default:
    @just --list

# Serve a game in release mode (e.g. `just run oregon-trail`)
run game:
    dx serve --release --debug-symbols=false --package {{game}}

# Serve a game in debug mode with hot reload (e.g. `just dev oregon-trail`)
dev game:
    dx serve --debug-symbols=false --package {{game}}

# Build a game's production wasm bundle (e.g. `just build oregon-trail`).
# --keep-names keeps readable panic backtraces despite dropping debug symbols.
build game:
    dx build --release --package {{game}} --debug-symbols=false --keep-names

# Per-game release shortcuts
oregon-trail:
    dx serve --release --debug-symbols=false --package oregon-trail

fort-nash:
    dx serve --release --debug-symbols=false --package fort-nash

dukedom:
    dx serve --release --debug-symbols=false --package dukedom

fur-trader:
    dx serve --release --debug-symbols=false --package fur-trader

hammurabi:
    dx serve --release --debug-symbols=false --package hammurabi

santa-paravia:
    dx serve --release --debug-symbols=false --package santa-paravia

taipan:
    dx serve --release --debug-symbols=false --package taipan

# Serve a minigames-kit demo crate (e.g. `just example timing_bar`)
example name:
    dx serve --debug-symbols=false --package minigames-kit-{{replace(name, "_", "-")}}

# Serve the SteadyHands minigame demo
steady-hands:
    dx serve --debug-symbols=false --package minigames-kit-steady-hands

# Serve the BucketBrigade minigame demo
bucket-brigade:
    dx serve --debug-symbols=false --package minigames-kit-bucket-brigade

# Serve the HotCold minigame demo
hot-cold:
    dx serve --debug-symbols=false --package minigames-kit-hot-cold

# Serve the Sequence minigame demo
sequence:
    dx serve --debug-symbols=false --package minigames-kit-sequence

# Run the kaintuck shared-economy game server (bots + humans trade one market).
# Native, no wasm — listens on ws://localhost:<port>/ws. To connect a human:
# `just build kaintuck`, add `<meta name="riparion-ws-base" content="ws://localhost:4317/ws">`
# to the bundle's index.html, serve it, and open it in a browser.
# e.g. `just server` or `just server 16 4317 200`
server bots="12" port="4317" tick_ms="300":
    BOTS={{bots}} PORT={{port}} TICK_MS={{tick_ms}} cargo run -p kaintuck-server

# Serve the kaintuck client wired to the shared-market server. Run `just server`
# in one terminal, `just play` in another, then open the printed URL. Builds the
# bundle if missing, points it at the server, and serves it — no manual steps.
# Force a fresh client build first with `just build kaintuck`.
# e.g. `just play` or `just play 4317 8123`
play port="4317" static_port="8123":
    #!/usr/bin/env bash
    set -euo pipefail
    bundle="{{justfile_directory()}}/target/dx/kaintuck/release/web/public"
    if [ ! -f "$bundle/index.html" ]; then
        echo "building client bundle (one-time)…"
        (cd "{{justfile_directory()}}/games/kaintuck" && dx build --release --debug-symbols=false --keep-names)
    fi
    # Wire the client to the server (idempotent).
    grep -q riparion-ws-base "$bundle/index.html" || \
        sed -i "s#</head>#<meta name=\"riparion-ws-base\" content=\"ws://localhost:{{port}}/ws\"></head>#" "$bundle/index.html"
    echo ""
    echo "  ▶ open  http://localhost:{{static_port}}"
    echo "    (shared market via ws://localhost:{{port}}/ws — make sure 'just server' is running)"
    echo ""
    cd "$bundle" && python3 -m http.server {{static_port}}

# End-to-end smoke test of the server (engine, sim, sockets, live browser).
# e.g. `just smoke` or `just smoke --quick`
smoke *args:
    scripts/smoke-server.sh {{args}}

# Run all workspace tests
test:
    cargo test --workspace
