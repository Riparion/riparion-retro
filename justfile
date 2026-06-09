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

# Serve a minigames-kit demo (e.g. `just example timing_bar`)
example name:
    dx serve --package minigames-kit --example {{name}}

# Serve the SteadyHands minigame demo
steady-hands:
    dx serve --package minigames-kit --example steady_hands

# Run all workspace tests
test:
    cargo test --workspace
