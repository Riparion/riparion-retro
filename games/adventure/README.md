# Colossal Cave Adventure

<img src="screenshot.png" width="280" alt="Colossal Cave Adventure — the debris room, lamp lit" align="right">

A mobile-first, WASM port of the original text adventure: Will Crowther &
Don Woods' **Colossal Cave Adventure** (the canonical 1977 **350-point**
version), built with [Dioxus](https://dioxuslabs.com) 0.7. Somewhere nearby is
Colossal Cave, where others have found fortunes in treasure and gold — though
it is rumored that some who enter are never seen again. Explore the
above-ground forest and the cave below, collect all fifteen treasures, and
deposit them in the well house, dodging dwarves, a knife-throwing pirate, a
dragon, a bear, and a toll-taking troll along the way.

The engine is a faithful, line-by-line reimplementation of the original
PDP-10 FORTRAN, ported from the well-documented reference
[`brandon-rhodes/python-adventure`](https://github.com/brandon-rhodes/python-adventure)
(Apache-2.0; the game text is public domain). It even reproduces **CPython's
Mersenne Twister** so the reference walkthroughs replay bit-for-bit — see
testing below. The canonical 12-section `advent.dat` is embedded and parsed at
startup.

## Playing

- **Type two-word commands** — `GO NORTH` (or just `N`), `TAKE LAMP`,
  `OPEN GRATE`, `WAVE ROD`, `XYZZY`, `DROP BIRD`. The parser reads the first
  five letters of each word, exactly like the original.
- **Autocomplete chips** — as you type, the vocabulary suggests matching words;
  tap one to fill it in. No physical keyboard required.
- **The lamp** — light it before you descend; its battery is finite, so don't
  dawdle (there's a vending machine for fresh batteries if you find the coins).
- **Magic words** — `XYZZY`, `PLUGH`, and `PLOVER` teleport you around once
  you've discovered where they work.
- **Scoring** — points for treasures seen, carried, and safely stowed, for
  surviving, and for reaching the cave's closing endgame. A local hall of fame
  records your best runs.
- **Auto-save** — persists to localStorage every turn; refresh and resume right
  where you left off.

## Development

```
src/
├─ engine/            # Pure game rules — no UI deps, fully host-tested
│  ├─ advent.dat      #   the verbatim 350-point data file (include_str!'d)
│  ├─ data.rs         #   parser → immutable room/object/vocab/message tables
│  ├─ rng.rs          #   a bit-exact reimplementation of CPython's random
│  ├─ state.rs        #   Mode, transcript Line, the pending yes/no tag
│  ├─ game.rs         #   the turn loop, travel, actors, verbs, scoring
│  ├─ text.rs         #   ALL-CAPS → lightly-modernized sentence case
│  └─ tests.rs        #   replays python-adventure's seeded walkthroughs
├─ storage/           # localStorage save + hall of fame
└─ ui/                # Thin Dioxus renderer over one Signal<Game>
   ├─ components/      #   transcript, command input, status bar
   └─ screens/         #   splash, play, game over
```

The strict engine/UI split (see the repo's `NOTES.md`) keeps all rules in pure,
testable Rust. `cargo test` replays four complete seeded walkthroughs from the
reference port and asserts the output matches **exactly**, command for command —
dwarf encounters, the pirate, the dragon, the troll bridge, death and
reincarnation, the cave-closing endgame, and the final score all included.

```
cargo test -p adventure          # engine suites incl. golden-master walkthroughs
cargo clippy -p adventure --all-targets
cd games/adventure && dx serve   # then open the printed localhost URL
```

Ported to mobile by Tony Bierman. After the 1977 Crowther & Woods original.
