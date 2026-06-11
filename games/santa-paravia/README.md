# Santa Paravia and Fiumaccio

<img src="screenshot.png" width="280" alt="Santa Paravia — the start-of-year report mid-reign" align="right">

A mobile-first, WASM port of **Santa Paravia and Fiumaccio**, George Blank's 1978
city-state sim, built with [Dioxus](https://dioxuslabs.com) 0.7. You rule a
15th-century Italian city-state: each year you trade grain and land, distribute
grain to feed your people, set three taxes and the severity of your justice, and
invest in marketplaces, mills, palaces, and cathedrals. Rule well and the
nobility raises your title — from Sir or Lady all the way to **King or Queen**,
which wins the game. But life was short: you're granted a reign of only 20–55
years, and a sprawling, poorly-defended city invites a raid from Baron Peppone.

The mechanics are a faithful reimplementation drawn from the C port at
[darkf/paravia](https://github.com/darkf/paravia) (itself from Thomas Knox's port
of Blank's original): the exact harvest, grain/land pricing, population, tax,
invasion, and title formulas. Two deliberate clean-ups: the title ladder is the
natural eight-rung `Sir → … → King` (the C port's off-by-one quietly skipped
Baron), and `Random(hi)` is a genuine uniform draw — the C port's
`rand()/RAND_MAX` integer-divides to nearly zero, collapsing its randomness — atop
retro-kit's seedable RNG.

## Playing

Each year runs through four decisions, then the realm reacts:

- **Market** — grain and land prices swing with the harvest. Buy grain cheap in
  good years and sell it dear in lean ones; buy land to widen your tax base.
- **Feed the city** — release enough grain to meet demand or your serfs starve;
  a generous surplus (about 1.3× demand) draws migrants to the city.
- **Fiscal policy** — customs, sales, and wealth taxes fill the treasury, but
  heavy taxes choke the growth of merchants and nobles. Harsh justice raises
  court fees yet drives serfs away and lowers your title. Projected revenue
  updates live as you tune the dials.
- **State purchases** — buildings raise your public works (the heart of the
  title score) and some pay yearly; platoons of soldiers keep the Baron at bay.

Other niceties:

- **Four difficulty tiers** — Apprentice to Grand Master scale how large a
  city-state each title demands.
- **How to Rule** — an in-game rules screen.
- **Touch-first UI** — big buttons, numeric entry, no keyboard needed.
- **Auto-save** — persists to localStorage every step; refresh and you resume
  exactly where you left off, mid-year.
- **Hall of Rulers** — a local high-score table. The original kept no score, so
  this one ranks reigns with the crown above every reign that fell short, then by
  years ruled, then the estate you leave behind.

## Development

```
src/
├─ engine/          # Pure game rules — no UI deps, fully unit-tested on host
│  ├─ state.rs      #   treasury/land/grain/populace, buildings, policy, modes
│  ├─ scoring.rs    #   synthesized score and the hall-of-fame ladder
│  ├─ interaction.rs #  the engine↔UI narration contract
│  ├─ mod.rs        #   Game struct, the year-phase state machine, all formulas
│  └─ tests.rs      #   spec-exact numbers + a multi-seed full-reign smoke test
├─ storage/         # localStorage save/resume + high scores
├─ ui/              # Dioxus components & screens (CRT theme)
└─ app.rs           # Signal<Game> root, mode dispatch, auto-save effect
```

The year's procedural loop is modelled as a serialized `Phase` cursor: a single
`run_phases` driver walks harvest → market → release → invasion → taxes →
purchases → title check → year's end, pausing on the four input screens and
queueing the year's narration so a refresh resumes mid-year in place.

```bash
dx serve              # develop (Tailwind compiles automatically)
cargo test            # engine test suite (runs on host, no browser)
cargo clippy --all-targets
dx build --release --debug-symbols=false --keep-names    # production wasm bundle
```

The CRT identity comes from `retro-kit`; Tailwind handles layout only.
