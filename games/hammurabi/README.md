# Hammurabi

A mobile-first WASM port of HAMURABI from *BASIC Computer Games* (1978),
after Doug Dyment's *The Sumer Game* (1968) — the original city-state
resource-management game. The missing M is original.

<img src="screenshot.png" width="260" alt="Hammurabi — a year-5 steward's report">

Govern ancient Sumeria for a ten-year term. Each year: trade land at
17–26 bushels per acre, feed your people (20 bushels a head), and plant
seed (one bushel sows two acres; each citizen tends ten). Then the steward
reports the harvest (1–5 bushels per acre), what the rats ate, who came to
the city — and whether the plague struck.

Every formula, threshold, and quirk matches the BASIC listing line for
line: rats strike only on even rolls and eat `INT(S/C)`, immigration is
`INT(C*(20A+S)/P/100+1)`, the plague is `INT(10*(2*RND-.3)) <= 0` per year
(~20%, whatever the listing's comment claims), starving more than 45% of
the city in one year gets you impeached as NATIONAL FINK, you can never
sell your last acre, and the four verdicts of history — fink, heavy-handed,
not-too-bad (with its would-be assassins), and Charlemange-grade fantastic —
use the exact `P1`/`L` thresholds. One concession to the soft keyboard: the
original ends the game on negative input; here amounts are clamped instead.

The original kept no score; the hall of rulers ranks honourable terms by
verdict tier plus the size of the city you leave behind. Reigns auto-save
and resume on reload.

```bash
dx serve            # develop
dx build --release --debug-symbols false --keep-names true  # production wasm bundle
cargo test -p hammurabi   # engine suite (host, no browser needed)
```
