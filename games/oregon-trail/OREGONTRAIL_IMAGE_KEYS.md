# Oregon Trail cover-art image keys

Drop a square `.jpg` into `assets/covers/` named for a screen or moment, rebuild,
and that art renders as the screen's cover. No code changes needed — `build.rs`
discovers the file and the `Cover` component renders it.

## Art spec

**Square 1:1**, recommend ~1024×1024. Format: **`.jpg`** (lowercase `.jpg`, not
`.jpeg`).

## How a key resolves

For the current screen the renderer builds an ordered list of candidate keys —
**most-specific narrative key first, then the general screen key** — and renders
the **first one that has a jpg**. If none exist, no cover renders.

So a general key is the floor (`steady.jpg` covers every steady-hand event), and
a narrative key **supersedes** it when present (`steady-snakebite.jpg` shows only
for the snakebite). You never have to provide both.

- General key = the screen's `Mode`, kebab-cased.
- Narrative key = `<general-key>-<variant>`.

## Discover keys by playing (debug builds)

In a debug build (`dx serve`), a small strip at the bottom of every screen lists
that screen's candidate keys, marking with `✓` the one that resolved to art.
Walk to any screen or event and read off exactly what to name the file. The strip
is compiled out of `--release`.

## General keys (one per screen — the fallback)

`splash` · `new-game` · `outfit` · `trail` · `fort` · `eat` · `shoot` · `hunt` ·
`flee` · `climb` · `fog` · `splint` · `dose` · `steady` · `brigade` · `riders` ·
`interaction` · `game-over`

## Narrative keys (supersede the general key)

### Trail hub by location (`trail`)
The fortnight hub shows location-specific art for the stretch of country you're
crossing (the same "where you are" bands shown on screen).
| Key | Stretch (miles) |
| --- | --- |
| `trail-kansas-plains` | The Kansas plains (0–319) |
| `trail-platte-river` | The Platte River valley (320–549) |
| `trail-fort-laramie` | Fort Laramie country (550–639) |
| `trail-high-plains` | The high plains (640–949) |
| `trail-south-pass` | The Rocky Mountains — South Pass (950–1699) |
| `trail-blue-mountains` | The Blue Mountains (1700–2039) |
| `trail-willamette-valley` | The Willamette Valley (2040+) |

### Steady-hand events (`steady`)
| Key | When |
| --- | --- |
| `steady-snakebite` | drawing snake venom |
| `steady-ford` | holding the wagon level fording a river |
| `steady-ox-leg` | wrapping a hurt ox's leg |

### Bucket-brigade events (`brigade`)
| Key | When |
| --- | --- |
| `brigade-fire` | fire in the wagon |
| `brigade-rains` | heavy rains, leaking load |
| `brigade-blizzard` | guttering fire in a blizzard pass |

### Gunfights (`shoot`)
| Key | When |
| --- | --- |
| `shoot-bandits` | bandits attack |
| `shoot-wild-animals` | wild animals attack |
| `shoot-riders-open` | hostile riders, wagons not circled |
| `shoot-riders-circled` | hostile riders, wagons circled |

### Riders ahead (`riders`)
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
| `interaction-wagon-wheel-breaks` | a wagon wheel breaks |
| `interaction-ox-wanders-off` | an ox wanders off |
| `interaction-son-lost` | your son gets lost |
| `interaction-unsafe-water` | unsafe water |
| `interaction-bandits-attack` | "Bandits attack!" beat (precedes the gunfight) |
| `interaction-wild-animals-attack` | "Wild animals attack!" beat |
| `interaction-hail-storm` | a hail storm |
| `interaction-helpful-indians` | helpful Indians share food |
| `interaction-cold-weather` | cold weather in the mountains |

Other interaction beats (mountain-pass results, hunt results, rider banter,
minigame-resolution lines) currently share the generic `interaction` key. To give
one its own art, tag its `message(...)` call as `message_keyed(text, "<slug>")` in
the engine, then add `interaction-<slug>.jpg`.

### Endings (`game-over`)
| Key | When |
| --- | --- |
| `game-over-victory` | reached Oregon City |
| `game-over-starved` | ran out of food |
| `game-over-pneumonia` | ran out of medical supplies |
| `game-over-snakebite` | snakebite with no medicine left |
| `game-over-winter` | too long on the trail; winter |
| `game-over-cant-afford-doctor` | illness with no cash for a doctor |
| `game-over-rider-massacre` | out of bullets against the riders |
| `game-over-wolves` | overpowered by wolves |

## Adding a cover

1. Export a square jpg.
2. Save it as `assets/covers/<key>.jpg` using a key from the tables above.
3. Rebuild (`dx serve` picks it up; `build.rs` reruns when the folder changes).

That's it — the screen now shows the cover. Add a narrative-key file later to
override the general one for a specific moment.
