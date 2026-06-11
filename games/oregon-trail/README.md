# The Oregon Trail

<img src="screenshot.png" width="280" alt="The Oregon Trail — the trail hub in the Rocky Mountains" align="right">

A mobile-first, WASM port of the 1975 MECC classic **The Oregon Trail** (by Don
Rawitsch, Bill Heinemann & Paul Dillenberger), built with
[Dioxus](https://dioxuslabs.com) 0.7. Outfit a wagon in Independence, Missouri
in the spring of 1847 and lead your family 2,040 miles to Oregon City before
winter buries the passes — hunting, fording rivers, trading at forts, and
surviving dysentery, snakebites, bandits, and blizzards along the way.

The mechanics are a faithful reimplementation of the original CDC Cyber BASIC
source ([clintmoyer/oregon-trail](https://github.com/clintmoyer/oregon-trail)):
the exact mileage formula, the cumulative event-probability table, the
eating/illness model, the river and mountain hazards, and the marksmanship
timing.

## Playing

- **Outfit your wagon** — split $700 across oxen, food, ammunition, clothing,
  and supplies. A stronger team travels faster; warm clothing survives the
  mountain blizzards; misc. supplies double as your medicine chest.
- **Each fortnight** — hunt, stop at a fort, or press on; then choose how well
  to eat (eating poorly stretches the larder but courts illness).
- **The shooting game** — hunts and gunfights flash a word; tap the matching
  one as fast as you can. Your reaction time *is* your aim (the original's
  typed-word timing, adapted for touch).
- **Touch-first UI** — big buttons, numeric entry, no keyboard needed.
- **Auto-save** — persists to localStorage every step; refresh and you resume
  exactly where you left off.
- **Trail Hall of Fame** — a local high-score table. The original kept no
  score, so this one synthesizes one from surviving supplies, distance, and how
  early you arrive.

## Development

```
src/
├─ engine/        # Pure game rules — no UI deps, fully unit-tested on host
│  ├─ state.rs    #   party supplies, the 1847 calendar, terrain, modes
│  ├─ events.rs   #   riders, the random-event table, mountains, illness, hunts
│  ├─ scoring.rs  #   synthesized score, ranks, arrival-date math
│  ├─ interaction.rs # the engine↔UI message/decision contract
│  └─ mod.rs      #   Game struct + the fortnight turn-flow state machine
├─ storage/       # localStorage save/resume + high scores
├─ ui/            # Dioxus components & screens (CRT theme)
└─ app.rs         # Signal<Game> root, mode dispatch, auto-save effect
```

```bash
dx serve              # develop (Tailwind compiles automatically)
cargo test            # engine test suite (runs on host, no browser)
cargo clippy --all-targets
dx build --release --debug-symbols=false --keep-names    # production wasm bundle
```

The CRT identity comes from `retro-kit`; game-specific flourishes (the trail
progress track) live in `assets/main.css`. Tailwind handles layout only.
