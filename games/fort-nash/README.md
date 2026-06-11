# Fort Nash

A mobile-first, WASM game built with [Dioxus](https://dioxuslabs.com) 0.7 on the
**Oregon Trail** engine. March with James Robertson's overland party down Daniel
Boone's Wilderness Road — from **Fort Patrick Henry** (Kingsport, TN) through the
**Cumberland Gap** to the **French Lick**, where they raised **Fort Nashborough**
(Nashville) on Christmas Day, **1779** — in the coldest winter in memory.

It is a re-skin of [`../oregon-trail`](../oregon-trail): the same proven turn-flow,
the same seven minigames, the same host-tested engine — re-themed to the 1779
frontier. Nine historical checkpoints replace the Oregon landmarks, weekly turns
replace fortnights, and a Cherokee war party, freezing sleet, frostbite, and a
signature **frozen-Cumberland ice crossing** replace dysentery and the Rockies.

## The nine checkpoints

Fort Patrick Henry → Moccasin Gap → Powell Valley → Martin's Station →
Cumberland Gap → Crab Orchard / Mansker's Station → the Kentucky barrens →
the Cumberland River → **Fort Nashborough**.

## Playing

- **Outfit your train** — split $700 across livestock, provisions, powder & shot,
  winter clothing, and supplies. A stronger train travels faster; blankets keep
  the party off the doctor's list in the deep cold; supplies double as the
  medicine chest.
- **Each week** — hunt, stop at a station (the blockhouse, Martin's, Mansker's),
  or press on; then choose how well to eat (eating poorly stretches the larder
  but courts illness in the cold).
- **A war party ahead** — run (a route-memory chase), fight (the rifle reaction
  game), press on, or fort up. They may not be what they appear.
- **The frozen Cumberland** — the Christmas crossing is a steady-hand trace:
  drive the livestock over the ice without breaking it. A badly shaky run drops
  the party through.
- **Auto-save** — persists to localStorage every step; refresh and you resume
  exactly where you left off.
- **Settlers' Roll** — a local high-score table, synthesized from surviving
  supplies, distance, and how early you reach the French Lick.

## Development

```
src/
├─ engine/        # Pure game rules — no UI deps, fully unit-tested on host
│  ├─ state.rs    #   party supplies, the 1779 winter calendar, checkpoints, modes
│  ├─ events.rs   #   war party, the random-event table, ridge passes, the ice crossing
│  ├─ scoring.rs  #   synthesized score, ranks, arrival-date math
│  ├─ interaction.rs # the engine↔UI message/decision contract
│  └─ mod.rs      #   Game struct + the weekly turn-flow state machine
├─ storage/       # localStorage save/resume + high scores
├─ ui/            # Dioxus components & screens (CRT theme)
└─ app.rs         # Signal<Game> root, mode dispatch, auto-save effect
```

```bash
dx serve              # develop (Tailwind compiles automatically)
cargo test            # engine test suite (runs on host, no browser)
cargo clippy --all-targets
dx build --release --debug-symbols false    # production wasm bundle
```

Cover art ships empty — drop square `trail-<checkpoint>.jpg` (and the per-event
keys) into `assets/covers/` and the build picks them up automatically; see
[`FORTNASH_IMAGE_KEYS.md`](FORTNASH_IMAGE_KEYS.md). The CRT identity comes from
`retro-kit`; the trail progress track lives in `assets/main.css`. Tailwind
handles layout only.
