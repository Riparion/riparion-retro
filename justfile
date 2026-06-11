# Riparion Retro — dev tasks. Run `just` to list recipes.

# List available recipes
default:
    @just --list

# Serve a game in release mode (e.g. `just run oregon-trail`)
run game:
    dx serve --release --package {{game}}

# Serve a game in debug mode with hot reload (e.g. `just dev oregon-trail`)
dev game:
    dx serve --package {{game}}

# Build a game's production wasm bundle (e.g. `just build oregon-trail`).
# --debug-symbols false works around dx 0.7.9's pinned binaryen (v127) choking
# on DWARF 5 (rustc emits it, wasm-opt SIGABRTs reading it, dx silently ships
# the UN-optimized wasm); --keep-names true keeps readable panic backtraces.
build game:
    dx build --release --package {{game}} --debug-symbols false --keep-names true

# Per-game release shortcuts
oregon-trail:
    dx serve --release --package oregon-trail

dukedom:
    dx serve --release --package dukedom

fur-trader:
    dx serve --release --package fur-trader

hammurabi:
    dx serve --release --package hammurabi

santa-paravia:
    dx serve --release --package santa-paravia

taipan:
    dx serve --release --package taipan

# Serve a minigames-kit demo crate (e.g. `just example timing_bar`)
example name:
    dx serve --package minigames-kit-{{replace(name, "_", "-")}}

# Serve the SteadyHands minigame demo
steady-hands:
    dx serve --package minigames-kit-steady-hands

# Serve the BucketBrigade minigame demo
bucket-brigade:
    dx serve --package minigames-kit-bucket-brigade

# Serve the HotCold minigame demo
hot-cold:
    dx serve --package minigames-kit-hot-cold

# Serve the Sequence minigame demo
sequence:
    dx serve --package minigames-kit-sequence

# Run all workspace tests
test:
    cargo test --workspace
