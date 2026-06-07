# Fur Trader

A mobile-first WASM port of FUR TRADER by Dan Bachor, as published in
*BASIC Computer Games* (1976) — lead a French fur-trading expedition out of
the Lake Ontario country in 1776.

<img src="screenshot.png" width="260" alt="Fur Trader — a year-3 expedition journal and sale ledger">

Each year you start with $600 in savings (the first year) and 190 fresh
furs to distribute among four pelts — mink, beaver, ermine, and fox. Then
you pick one of three forts to trade at, each with its own prices, supply
costs, and dangers:

- **Fort Hochelaga (Montreal)** — the easiest route, but far from any
  seaport: low fur prices, the highest supply cost ($160), and no perils.
- **Fort Stadacona (Quebec)** — an average trade, but you must portage and
  cross the Lachine Rapids ($140). Your beaver can be stolen at the
  portage, your fox can spoil, or your canoe can upset and lose everything.
- **Fort New York** — Dutch-controlled, through Iroquois land ($105): the
  highest fur prices and the cheapest supplies, but a war party may take
  all your furs — or kill the whole expedition and end the game.

Prices and perils match the BASIC listing line for line: each fort rolls
its pelt prices with the original `INT((a*RND+b)*100+.5)/100` formulas in
the original order, then rolls the trip event on `P=INT(10*RND)+1` with its
20% / 40% / 20% / 20% ladder. The trip is paid for up front — a wrecked or
fatal expedition still costs you. Fort New York's "highest value" sells
ermine and beaver at the high start-of-year prices, exactly as the original
intends.

## Deviations from the 1976 listing

- **Fort Stadacona fox price.** The original never sets a fox price at
  Stadacona (lines 1205–1207 roll only mink, ermine, and beaver), so fox
  would sell at a stale price — or $0.00 on a first-ever Stadacona trip.
  This port rolls a sensible fox price there (`$0.80–$1.00`), as most
  modern ports do.
- **Endings and a score.** The original loops forever, ending only at the
  Iroquois ambush. Here the otherwise-endless year loop gains two endings:
  **Retire** banks your savings and enters the local hall of traders, and
  going **bankrupt** (unable to raise the $105 for even the cheapest
  expedition) ends the run. Final savings *is* the score — the only one the
  original implies.
- **Allocation.** Furs are distributed with a clamped numeric keypad, so
  the original's "you may not have that many furs / do not try to cheat"
  restart can't happen. Taking fewer than 190 is still allowed.

Runs auto-save and resume on reload.

```bash
dx serve                   # develop
dx build --release         # production wasm bundle
cargo test -p fur-trader   # engine suite (host, no browser needed)
```
