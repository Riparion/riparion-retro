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

# Run all workspace tests
test:
    cargo test --workspace
