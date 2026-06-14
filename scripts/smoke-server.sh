#!/usr/bin/env bash
#
# End-to-end smoke test for the kaintuck shared-economy server.
#
# Tiers (each gates the next):
#   1. engine golden-trace + sim unit tests
#   2. sim CLI — headless economy under load (writes a JSON report)
#   3. server socket tests — real /ws round-trip (Join/Welcome, TradeOrder/Fill,
#      broadcast price move), no browser
#   4. live browser — the real wasm client connects to a running server and
#      exchanges shared-market messages (Join sent, Welcome + MarketDelta received)
#
# Usage:
#   scripts/smoke-server.sh [--quick] [--no-build]
#     --quick     tiers 1-3 only (skip the browser tier)
#     --no-build  reuse an existing dx release bundle (skip the dx build)
#
# Env overrides: PORT (4317) STATIC_PORT (8123) BOTS (8) TICK_MS (300)
#                CHROMIUM (/usr/bin/chromium)
#
# Run from anywhere; resolves the repo root itself.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

PORT="${PORT:-4317}"
STATIC_PORT="${STATIC_PORT:-8123}"
BOTS="${BOTS:-8}"
TICK_MS="${TICK_MS:-300}"
CHROMIUM="${CHROMIUM:-/usr/bin/chromium}"
BUNDLE="$REPO/target/dx/kaintuck/release/web/public"
NODE_DIR="$REPO/target/smoke-node"

QUICK=0
BUILD=1
for a in "$@"; do
  case "$a" in
    --quick) QUICK=1 ;;
    --no-build) BUILD=0 ;;
    -h|--help) sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown arg: $a (try --help)"; exit 2 ;;
  esac
done

PIDS=()
cleanup() { for p in "${PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done; }
trap cleanup EXIT

say()  { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; exit 1; }

# --- Tier 1: engine ---------------------------------------------------------
say "Tier 1 — engine golden trace + sim tests"
if cargo test -p kaintuck-engine --features sim >/tmp/kt_t1.log 2>&1; then
  grep -E "golden_trace_is_stable|test result:" /tmp/kt_t1.log | head -3
else
  tail -25 /tmp/kt_t1.log; fail "engine tests (see /tmp/kt_t1.log)"
fi

# --- Tier 2: sim CLI --------------------------------------------------------
say "Tier 2 — sim CLI (headless economy under load)"
cargo run -q -p kaintuck-engine --features cli --bin sim -- \
  --bots 32 --ticks 5000 --policy mixed --format json --out /tmp/kt_sim_report.json
echo "report -> /tmp/kt_sim_report.json"

# --- Tier 3: server socket tests --------------------------------------------
say "Tier 3 — server socket tests (real /ws round-trip)"
if cargo test -p kaintuck-server >/tmp/kt_t3.log 2>&1; then
  grep -E "websocket_join|a_human_buy|test result:" /tmp/kt_t3.log | head
else
  tail -25 /tmp/kt_t3.log; fail "server socket tests (see /tmp/kt_t3.log)"
fi

if [ "$QUICK" = "1" ]; then
  say "DONE — tiers 1-3 passed (--quick skipped the browser tier)"
  exit 0
fi

# --- Tier 4: live browser ---------------------------------------------------
say "Tier 4 — live browser client <-> running server"

if ! command -v node >/dev/null 2>&1 || [ ! -x "$CHROMIUM" ]; then
  echo "node or chromium ($CHROMIUM) missing — skipping browser tier."
  echo "Set CHROMIUM=/path/to/chromium to run it."
  exit 0
fi

if [ "$BUILD" = "1" ] || [ ! -f "$BUNDLE/index.html" ]; then
  say "building release bundle (dx build --release --debug-symbols false)"
  if ! ( cd games/kaintuck && dx build --release --debug-symbols false ) >/tmp/kt_dx.log 2>&1; then
    tail -8 /tmp/kt_dx.log; fail "dx build (see /tmp/kt_dx.log)"
  fi
fi

# Inject the ws endpoint the wasm client reads — idempotent. (The CMS does this
# at serve time in production; here we patch the static bundle's HTML.)
if ! grep -q 'riparion-ws-base' "$BUNDLE/index.html"; then
  sed -i "s#</head>#<meta name=\"riparion-ws-base\" content=\"ws://localhost:${PORT}/ws\"></head>#" "$BUNDLE/index.html"
  echo "injected riparion-ws-base meta into bundle index.html"
fi

# Start the game server.
BOTS="$BOTS" PORT="$PORT" TICK_MS="$TICK_MS" cargo run -q -p kaintuck-server >/tmp/kt_server.log 2>&1 &
PIDS+=($!)
# Start the static file server for the bundle.
( cd "$BUNDLE" && python3 -m http.server "$STATIC_PORT" ) >/tmp/kt_static.log 2>&1 &
PIDS+=($!)

# Wait for both to come up.
for _ in $(seq 1 40); do curl -sf "http://localhost:${PORT}/health" >/dev/null 2>&1 && break || sleep 0.5; done
curl -sf "http://localhost:${PORT}/health" >/dev/null 2>&1 || fail "server did not start (see /tmp/kt_server.log)"
for _ in $(seq 1 40); do curl -sf -o /dev/null "http://localhost:${STATIC_PORT}/" 2>/dev/null && break || sleep 0.5; done
curl -sf -o /dev/null "http://localhost:${STATIC_PORT}/" 2>/dev/null || fail "static server did not start"
echo "server up on :$PORT, bundle on :$STATIC_PORT"

# Playwright-core (cached under target/, gitignored).
mkdir -p "$NODE_DIR"
if [ ! -d "$NODE_DIR/node_modules/playwright-core" ]; then
  say "installing playwright-core (one-time, into target/smoke-node)"
  ( cd "$NODE_DIR" && npm install playwright-core ) >/tmp/kt_pw.log 2>&1 || fail "playwright-core install (see /tmp/kt_pw.log)"
fi

# Browser drive script: inject the meta before boot (belt-and-suspenders with the
# HTML patch above), capture WebSocket frames, assert the protocol round-trip.
cat > "$NODE_DIR/smoke-browser.mjs" <<'SMOKE_EOF'
const { chromium } = (await import(process.env.PW)).default;
const APP = process.env.APP_URL;
const WS = process.env.WS_URL;

const browser = await chromium.launch({ executablePath: process.env.CHROMIUM, args: ['--no-sandbox'] });
const page = await browser.newPage();

await page.addInitScript((url) => {
  const add = () => {
    if (!document.head) return false;
    if (!document.querySelector('meta[name="riparion-ws-base"]')) {
      const m = document.createElement('meta');
      m.setAttribute('name', 'riparion-ws-base');
      m.setAttribute('content', url);
      document.head.appendChild(m);
    }
    return true;
  };
  if (!add()) { const o = new MutationObserver(() => { if (add()) o.disconnect(); }); o.observe(document, { childList: true, subtree: true }); }
}, WS);

const sent = [], received = [];
let wsUrl = null;
page.on('websocket', (ws) => {
  wsUrl = ws.url();
  ws.on('framesent', (f) => { try { sent.push(JSON.parse(f.payload)); } catch {} });
  ws.on('framereceived', (f) => { try { received.push(JSON.parse(f.payload)); } catch {} });
});

await page.goto(APP, { waitUntil: 'load' });
await page.waitForTimeout(6000);

const kinds = (a) => { const c = {}; for (const m of a) { const k = (m && typeof m === 'object') ? Object.keys(m)[0] : '?'; c[k] = (c[k] || 0) + 1; } return c; };
console.log('WS_URL=' + wsUrl);
console.log('SENT=' + JSON.stringify(kinds(sent)));
console.log('RECV=' + JSON.stringify(kinds(received)));

await browser.close();
const ok = wsUrl === WS && sent.some((m) => m && m.Join) && received.some((m) => m && m.Welcome) && received.some((m) => m && m.MarketDelta);
console.log(ok ? 'RESULT=PASS' : 'RESULT=FAIL');
process.exit(ok ? 0 : 1);
SMOKE_EOF

say "driving headless chromium against the live server"
if PW="$NODE_DIR/node_modules/playwright-core/index.js" \
   APP_URL="http://localhost:${STATIC_PORT}/" \
   WS_URL="ws://localhost:${PORT}/ws" \
   CHROMIUM="$CHROMIUM" \
   node "$NODE_DIR/smoke-browser.mjs"; then
  say "DONE — all tiers passed ✅"
else
  fail "browser tier (wasm client did not complete the protocol round-trip)"
fi
