# Fort Nash cover-art image keys

Drop a square `.jpg` into `assets/covers/` named for a screen or moment, rebuild,
and that art renders as the screen's cover. No code changes needed — `build.rs`
discovers the file and the `Cover` component renders it. Fort Nash ships with **no**
cover art; every screen falls back cleanly to the CRT layout until art is added.

## Art spec

**Square 1:1**, recommend ~1024×1024. Format: **`.jpg`** (lowercase `.jpg`, not
`.jpeg`).

## How a key resolves

For the current screen the renderer builds an ordered list of candidate keys —
**most-specific narrative key first, then the general screen key** — and renders
the **first one that has a jpg**. If none exist, no cover renders.

So a general key is the floor (`sequence.jpg` covers every order-memory event), and
a narrative key **supersedes** it when present (`sequence-frostbite.jpg` shows only
for the frostbite). You never have to provide both.

- General key = the screen's `Mode`, kebab-cased.
- Narrative key = `<general-key>-<variant>`.

## Discover keys by playing (debug builds)

In a debug build (`dx serve`), a small strip at the bottom of every screen lists
that screen's candidate keys, marking with `✓` the one that resolved to art. Walk
to any screen or event and read off exactly what to name the file. The strip is
compiled out of `--release`.

## General keys (one per screen — the fallback)

`splash` · `new-game` · `outfit` · `trail` · `fort` · `eat` · `shoot` · `hunt` ·
`flee` · `climb` · `fog` · `splint` · `dose` · `steady` · `brigade` · `sequence` ·
`riders` · `interaction` · `game-over`

## Narrative keys (supersede the general key)

### Trail hub by checkpoint (`trail`)
The weekly hub shows location-specific art for the checkpoint of the Wilderness
Road you're crossing (the same "where you are" bands shown on screen).
| Key | Checkpoint (miles) |
| --- | --- |
| `trail-fort-patrick-henry` | Fort Patrick Henry (0–179) |
| `trail-moccasin-gap` | Moccasin Gap (180–359) |
| `trail-powell-valley` | Powell Valley (360–559) |
| `trail-martins-station` | Martin's Station (560–759) |
| `trail-cumberland-gap` | The Cumberland Gap (760–999) |
| `trail-crab-orchard` | Crab Orchard / Mansker's Station (1000–1199) |
| `trail-kentucky-barrens` | The Kentucky barrens (1200–1399) |
| `trail-cumberland-river` | The Cumberland River (1400–1599) |
| `trail-fort-nashborough` | Fort Nashborough (1600+) |

### Steady-hand events (`steady`)
| Key | When |
| --- | --- |
| `steady-ice` | driving the livestock over the frozen Cumberland (Christmas crossing) |

### Bucket-brigade events (`brigade`)
| Key | When |
| --- | --- |
| `brigade-fire` | fire in camp |
| `brigade-rains` | freezing sleet soaking the load |
| `brigade-blizzard` | guttering fire in a blizzard pass |

### Sequence (order-memory) events (`sequence`)
| Key | When |
| --- | --- |
| `sequence-wheel` | re-seating a broken cart wheel (jack → block → bolt → seat) |
| `sequence-ox-leg` | dressing a lame animal's leg (wrap → pad → bind) |
| `sequence-frostbite` | the frostbite first-aid steps (warm → wrap → bind) |

### Gunfights (`shoot`)
| Key | When |
| --- | --- |
| `shoot-bandits` | outlaws block the trace |
| `shoot-wild-animals` | wolves out of the timber |
| `shoot-riders-open` | hostile war party, not forted up |
| `shoot-riders-circled` | hostile war party, forted up |

### War party ahead (`riders`)
| Key | When |
| --- | --- |
| `riders-hostile` | riders that look hostile |
| `riders-friendly` | riders that look friendly |

### Dosing / illness (`dose`)
| Key | When |
| --- | --- |
| `dose-mild` | mild illness |
| `dose-bad` | bad illness |
| `dose-serious` | serious illness |

### Trail incidents (`interaction`)
These are the tap-to-continue narration beats.
| Key | When |
| --- | --- |
| `interaction-livestock-strays` | a pack animal wanders off |
| `interaction-child-lost` | a child strays from the column |
| `interaction-unsafe-water` | bad water |
| `interaction-outlaws-attack` | "Outlaws block the trace!" beat (precedes the gunfight) |
| `interaction-wolves-attack` | "Wolves come out of the timber!" beat |
| `interaction-creek-ford` | a swollen creek crossing |
| `interaction-sleet-storm` | a sleet storm |
| `interaction-long-hunters` | long hunters share a cache of game |
| `interaction-cold-weather` | a hard freeze sets in |
| `interaction-ice-broke` | the ice cracks on the Cumberland (precedes the ending) |

Other interaction beats (pass results, hunt results, war-party banter,
minigame-resolution lines) currently share the generic `interaction` key. To give
one its own art, tag its `message(...)` call as `message_keyed(text, "<slug>")` in
the engine, then add `interaction-<slug>.jpg`.

### Endings (`game-over`)
| Key | When |
| --- | --- |
| `game-over-victory` | reached the French Lick / raised Fort Nashborough |
| `game-over-starved` | ran out of provisions |
| `game-over-pneumonia` | ran out of medicine; pneumonia in the cold |
| `game-over-frostbite` | frostbite with no medicine left |
| `game-over-winter` | too long on the road; the deep winter |
| `game-over-cant-tend-sick` | sickness with nothing left to trade for care |
| `game-over-raid-massacre` | out of powder against the war party |
| `game-over-wolves` | overpowered by wolves |
| `game-over-ice-broke` | the ice gave way on the Cumberland |

## Adding a cover

1. Export a square jpg.
2. Save it as `assets/covers/<key>.jpg` using a key from the tables above.
3. Rebuild (`dx serve` picks it up; `build.rs` reruns when the folder changes).

That's it — the screen now shows the cover. Add a narrative-key file later to
override the general one for a specific moment.
