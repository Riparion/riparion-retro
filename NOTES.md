# NOTES — How to build a game in this repo

Instructions for future sessions. The Oregon Trail (`games/oregon-trail/`) is the
reference implementation; when in doubt, read how it does it. These conventions
were all verified working on 2026-06-07 with Dioxus 0.7.1 / dx 0.7.9.

**Version bumps.** When a change modifies `crates/retro-kit` or any existing game
crate (e.g. extracting a shared component and refactoring callers onto it), rev
the **patch** number of every affected crate in its `Cargo.toml` (`0.1.0` →
`0.1.1`, …). A brand-new game crate starts at `0.1.0` and isn't a "modification".
So a change that adds a retro-kit component and rewires two games bumps retro-kit
*and* both of those games.

## The contract

Every game here is a **mobile-first, fully client-side WASM Dioxus web app**
with the shared **green-phosphor CRT aesthetic** from `crates/retro-kit`:
touch-first (≥44px targets, numeric soft keyboard, no physical keyboard
required), auto-saving to localStorage, with a local high-score table.

## Scaffold a new game

```bash
scripts/new-game.sh <name>      # then: cd games/<name> && dx serve
```

The script creates everything below with retro-kit already wired — the CRT
splash renders immediately. Don't use `dx new`: it generates a standalone
template (pinned deps, router demo) that fights these conventions. The
manual steps, for reference:

1. `mkdir -p games/<name>/{src,assets}` — the workspace glob
   (`members = ["crates/*", "games/*"]`) picks it up automatically.
2. `games/<name>/Cargo.toml` — copy from oregon-trail; inherit everything:

   ```toml
   [package]
   name = "<name>"
   version = "0.1.0"
   authors.workspace = true
   edition.workspace = true
   license.workspace = true

   [dependencies]
   retro-kit = { workspace = true }
   dioxus = { workspace = true }
   serde = { workspace = true }
   gloo-timers = { workspace = true }   # only if you need timed pacing

   [features]
   default = ["web"]
   web = ["dioxus/web"]
   ```

   New shared dependencies go in root `[workspace.dependencies]`, never
   pinned in a game crate.
3. `games/<name>/Dioxus.toml`: minimal `[application]` + `[web.app] title`.
4. `games/<name>/tailwind.css` containing `@import "tailwindcss";` plus
   `@source "../../crates/retro-kit/src";` (the scan only covers the game's
   own sources — without the @source, utilities used only inside retro-kit
   components silently vanish from the build), and an empty
   `games/<name>/assets/tailwind.css` — dx auto-compiles input → output.
5. App root (see `games/oregon-trail/src/app.rs`) must link, in this order:

   ```rust
   document::Stylesheet { href: retro_kit::CRT_CSS }  // shared identity
   document::Stylesheet { href: MAIN_CSS }            // game-specific extras
   document::Stylesheet { href: TAILWIND_CSS }
   document::Meta { name: "viewport",
       content: "width=device-width, initial-scale=1, viewport-fit=cover, user-scalable=no" }
   ```

## Architecture (the pattern that works)

**Strict engine/UI split.** `src/engine/` is pure Rust — no dioxus, no
web-sys, no gloo. All rules, all randomness, all state transitions. It must
be exhaustively testable with `cargo test` on the host. The UI is a thin
renderer that calls engine methods and never duplicates rules (it may clamp
inputs for UX, but the engine re-clamps everything).

**One `Signal<Game>` via context.** The whole game state — including which
screen is showing (`mode: Mode` enum), pending prompts, and the RNG — lives
in a single serializable `Game` struct. The root component provides it with
`use_context_provider`, matches on `mode`, and renders exactly one screen.
**No router** — browser history desyncs from game state; navigation is
`game.write().mode = ...`.

**Interaction queue for event sequences.** When the game needs to present a
chain of messages/prompts (taipan's arrival events), the engine pushes typed
`Interaction` values onto a `VecDeque`; a host screen renders the head and
feeds the player's `Response` back into `engine.resolve(response)`. Stage
multi-step chains so later steps see earlier steps' state changes. Paced
"tap to continue" messages are just `Interaction::Message(String)`.

**Timed pacing via spawn + gloo-timers.** Engine resolves a turn
synchronously and returns `Vec<String>` messages; the UI plays them back:

```rust
spawn(async move {
    let msgs = game.write().take_turn(input);   // borrow ends at the `;`
    for m in msgs {
        log.write().push(m);
        TimeoutFuture::new(550).await;          // NO borrow held here
    }
    if game.peek().turn_decided() { game.write().finish_turn(); }
});
```

Keep the mode unchanged until pacing finishes (store the outcome in the
engine, apply it in a `finish_*()` call) so the screen doesn't flip away
mid-playback.

**Persistence.** Use `retro_kit::storage` with keys `"<name>.save"` /
`"<name>.highscores"` and a per-game `SAVE_VERSION` const (bump it on any
breaking serde change — old saves are then silently discarded, by design).
Auto-save from one `use_effect` in the app root that reads the game signal;
on game over: clear the save and `record_score` exactly once (guard with a
`recorded` flag inside the game state).

**Start screen (required).** Every game must open on its title/splash screen
on every load — never drop the player straight into a resumed game. A save
loads in a gameplay `Mode`, so gate the splash with a session signal rather
than the `mode`: `let on_splash = use_signal(|| true)` in the app root,
provided via context; the root renders `Splash` whenever `on_splash()` is
true (and hides the `StatusBar`), otherwise it matches on `mode`. The splash
detects a resumable save with `game.peek().mode != Mode::Splash` and, when
true, shows **RESUME** (just `on_splash.set(false)` — the loaded `mode`
renders) alongside **NEW GAME** (the normal start action + `on_splash.set
(false)`); a fresh game shows only the single start button. Keep the start
action in one button whose label/class varies by `resumable` — a closure
shared across two `if`-branch buttons fails to compile (closures aren't
`Copy`, so they can't move into two `onclick`s). The auto-save effect is
unaffected: while the resume splash shows, `mode` is still the gameplay mode,
so the existing save arm re-persists the identical state harmlessly.

The splash also ends with two muted footer lines: the period attribution
(`text-xs opacity-60`, "After … as published in …"), then a porting credit
`p { class: "text-xs opacity-50", "Ported to mobile by Tony Bierman" }` —
include both on every game's title screen.

## Dioxus 0.7 pitfalls (each of these bit once)

- **Never hold a signal borrow across `.await`** — root `clippy.toml` makes
  it an error, and it panics at runtime. Pattern above; `peek()` (not
  `read()`) inside spawned tasks.
- `getrandom` needs its `js` feature on wasm — already wired via retro-kit;
  don't add a direct `rand` dependency to a game, use `retro_kit::rng`.
- `asset!()` paths are manifest-relative starting with `/` (e.g.
  `asset!("/assets/main.css")`). dx bundles `asset!()` files from dependency
  crates too (that's how `CRT_CSS` works).
- Closures that call `signal.write()` (directly or transitively) must be
  bound `let mut f = move |…|`.
- Layout: `h-[100dvh]` not `100vh`; `env(safe-area-inset-*)` on top/bottom
  bars (`.status-bar`/`.action-bar` from crt.css already do this).
- Integer money math: prefer `x + (x*rate).floor()` over `(x*(1+rate)).floor()`
  — `1000.0 * 1.005` is `1004.999…` in f64.

## The aesthetic

- All identity comes from `retro-kit/assets/crt.css`: palette CSS vars,
  scanlines/flicker (reduced-motion aware), `.crt` app frame, `.crt-btn`,
  `.crt-btn-primary`, `.crt-input`, `.crt-panel`, `.chip`, `.chip-label`,
  `.chip-danger`, `.status-bar`, `.action-bar`, `.splash-title`.
- Tailwind is for **layout only** (flex/grid/spacing/overflow). Game css is
  for game-specific flourishes, layered on the shared vars
  (`var(--phosphor)` etc.). Never fork the palette in a game.
- Reusable UI: `retro_kit::components::number_entry::NumberEntry` for all
  quantity/amount input (numeric soft keyboard + "All" button + live cost
  preview; omit `on_cancel` to hide the Cancel button), `chip::Chip` for
  status-bar chips (label/value + `danger`), and `stat_row::StatRow` for
  label/value reckoning rows. Number display: `retro_kit::format`
  (`fmt_money`, `group_thousands`, `fmt_num`). Timed message playback:
  `retro_kit::playback::play_paced` (await it inside the spawned task; the
  finish logic stays game-side). If you find yourself copying a component
  from another game, generalize it **into retro-kit** instead.

## Porting a classic? Spec first

The Oregon Trail's quality came from extracting a complete mechanics spec from
the original source *before* writing code: every formula, probability, and
threshold, with the RNG idiom pinned down (`R(x) = floor(uniform·x)`;
`R(n)==0` ⇒ 1-in-n). Do the same: fetch the original source, have a subagent
produce an implementation-ready spec, put the spec in the plan, then test
engine functions against the spec's exact numbers (price bounds, interest,
score tables) with seeded RNG.

## Verify before declaring done

1. `cargo test` (root) — engine suites, deterministic via `GameRng::from_seed`.
   Include a multi-seed scripted smoke test that plays whole games through
   the public engine API and asserts invariants (cash ≥ 0, no wedged modes).
2. `cargo clippy --all-targets` — keep it at zero warnings.
3. `dx serve --port 8899 --interactive false` from the game dir (no
   `--headless` flag exists); poll the port with curl, don't sleep.
4. **Drive it in a real browser** at 390×844: `npm install playwright-core`
   in a temp dir, `chromium.launch({ executablePath: '/usr/bin/chromium',
   args: ['--no-sandbox'] })`. Play a representative loop, screenshot, and
   **look at the screenshots**. Check console errors. Test save/resume by
   reloading mid-game, and the game-over path (edit the localStorage save
   via `page.evaluate` to set up endgame states cheaply).
5. `dx build --release`, serve `target/dx/<name>/release/web/public` with
   `python3 -m http.server`, smoke once. (A 404 on `/favicon.ico` is the
   browser's default probe — benign; the real icon is a hashed asset.)

## Ship a screenshot

Every game must include **`games/<name>/screenshot.png`**: an *in-game*
capture (a representative play screen, not the splash), taken at the mobile
viewport (390×844, `deviceScaleFactor: 2` for crispness) using the
playwright-core recipe above. Sculpt a lived-in state first so the shot
shows the game being played — edit the localStorage save via
`page.evaluate` (envelope is `{version, data}`; set `data.state.*` and
`data.mode`, then reload). Embed it in the game's README and add it to the
table in the root README (`<img src="games/<name>/screenshot.png"
width="120">`). Re-capture whenever the look changes materially.

## Icons

Every game ships a PWA/home-screen icon set in `games/<name>/assets/`, all in
the CRT palette (`#020402` field, `#33ff66` phosphor glyph, faint glow):

- `icon.svg` — scalable master / favicon source (the `any` framing).
- `icon-192.png`, `icon-512.png` — `purpose: any`; transparent rounded tile
  with a dim keyline.
- `icon-192-maskable.png`, `icon-512-maskable.png` — `purpose: maskable`;
  **opaque, full-bleed**, glyph kept inside the adaptive safe zone (inner
  ~80%, the radius-40% circle) so Android's circle/squircle crop never clips
  it.
- `apple-touch-icon.png` — 180×180, **opaque, full-bleed, square** (iOS
  ignores the manifest for Add-to-Home-Screen, composites transparency on
  black, and rounds the corners itself).

**The one required file is `games/<name>/assets/icon-512.png`.** riparion-cms's
builder auto-discovers it at build (priority `icon-512.png` → `icon.png` →
`apple-touch-icon.png`, searched in the game's `assets/` then the repo root),
materializes it into the media library at publish, and serves it as the run
surface's manifest + apple-touch icon and the Launch-button thumbnail. PNG only
(iOS won't take an SVG apple-touch icon). No `icon-512.png` ⇒ the game still
runs, just with no custom icon. The other files in the set are derived for
completeness / future use.

The glyph masters and the render pipeline live in `scripts/gen-icons.mjs`
(authored in a 0..100 design space, rasterised with headless Chromium for
SVG-filter/glow fidelity — same playwright-core recipe as the screenshots).
Regenerate the whole set with `node scripts/gen-icons.mjs`.

**Adding a game's icon** (the only manual step — authoring the silhouette is a
small creative act the tooling can't do for you):

1. Add a `<game-dir>: { adj: 1.0, svg: \`…\` }` entry to the `GLYPHS` map in
   `scripts/gen-icons.mjs` — one bold silhouette in the `#33ff66` phosphor,
   drawn to fill roughly the central 80% of the 0..100 canvas (use the four
   existing glyphs as the model; keep it legible at 48px, no fine detail).
2. `node scripts/gen-icons.mjs --sheet` — writes the six files into the game's
   `assets/` and a `/tmp/icon-contact-sheet.png` QA sheet.
3. **Look at the sheet.** Each row shows any / maskable (with the Android
   adaptive crop circle overlaid) / apple-touch. Confirm the glyph reads at
   size and sits inside the crop circle; tweak the glyph (or its `adj` scale)
   and re-run until it does.

**The manifest is not in the bundle.** dx content-hashes anything routed
through `asset!()` (that's why bare `/favicon.ico` 404s — the real one is
hashed), so a manifest can't reference stable icon URLs from inside the
bundle. riparion-cms serves each game standalone at `/apps/<slug>/run/` (a
top-level document, not an iframe — verified in its `reader/apps.rs` /
`apps_static.rs`) and already rewrites that `index.html` + owns the canonical
apps origin and the app's display name. So the CMS is the right owner of the
per-slug manifest (`name`, `start_url`/`scope` = `/apps/<slug>/run/`, shared
`theme_color`/`background_color`) and the `<head>` links (`manifest`,
`apple-touch-icon`, `theme-color`); this repo only provides the art.
