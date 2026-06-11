# Taipan!

<img src="screenshot.png" width="280" alt="Taipan — the Comprador's report at Hong Kong" align="right">

A mobile-first, WASM port of Art Canfil's 1982 Apple ][ classic **Taipan**,
built with [Dioxus](https://dioxuslabs.com) 0.7. Trade opium, silk, arms and
general cargo across seven ports of the South China Sea — dodging Li Yuen's
pirates, Elder Brother Wu's compounding debt, storms, and the occasional
mugging — until you retire a millionaire (or go down with the ship).

The game mechanics are a faithful reimplementation of the original Applesoft
BASIC source (<https://taipangame.com/BASIC.txt>): exact price formulas,
event probabilities, combat math, and scoring.

## Playing

- **Touch-first UI** — big buttons, numeric keypad entry, no keyboard needed.
- **Auto-save** — the game persists to localStorage on every move; refresh
  and you're exactly where you left off.
- **Hall of Taipans** — local high-score table, scored like the original:
  `net worth / 100 / months^1.1`.

## Development

```
src/
├─ engine/       # Pure game rules — no UI deps, fully unit-tested on host
│  ├─ state.rs   #   world state, price tables, money formatting
│  ├─ prices.rs  #   the original price-generation formulas
│  ├─ events.rs  #   voyages (pirates/storms) + per-arrival event chain
│  ├─ combat.rs  #   sea battle resolution
│  ├─ rng.rs     #   seedable, serializable BASIC-style `FN R(X)`
│  └─ scoring.rs #   ranks and final score
├─ storage/      # localStorage save/resume + high scores
├─ ui/           # Dioxus components & screens (CRT theme)
└─ app.rs        # Signal<Game> root, mode dispatch, auto-save effect
```

```bash
dx serve              # develop (Tailwind compiles automatically)
cargo test            # engine test suite (runs on host, no browser)
cargo clippy --all-targets
dx build --release --debug-symbols=false --keep-names    # production wasm bundle
```

The retro CRT theme lives in `assets/main.css`; Tailwind (auto-compiled by
`dx serve` from `tailwind.css`) handles layout only.
