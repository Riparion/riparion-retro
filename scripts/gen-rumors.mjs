#!/usr/bin/env node
// Regenerate the dockside-rumor prose corpus in kaintuck.ron with an LLM.
//
// The rumor MECHANIC lives in the Rust engine (crates/kaintuck-engine/src/rumor.rs);
// this script only (re)authors the TEXT — the band-keyed tip templates and the
// held/wind payoff lines — and splices them into the `rumors:` block of
// crates/kaintuck-engine/src/kaintuck.ron. The committed RON is the source of
// truth (the game builds offline with no LLM); run this only to expand or
// refresh the writing, then `cargo test -p kaintuck-engine` to validate.
//
// Sources + reliabilities are gameplay tuning, not prose, so they're held fixed
// here rather than generated. Templates may use only {good} and {town}.
//
// Usage:  ANTHROPIC_API_KEY=sk-... node scripts/gen-rumors.mjs   (or `just gen-rumors`)

import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const RON = resolve(ROOT, 'crates/kaintuck-engine/src/kaintuck.ron');
const MODEL = process.env.RUMOR_MODEL ?? 'claude-opus-4-8';

// NOTE: the `sources:` list (keys, voices, reliabilities) is gameplay tuning and
// lives in kaintuck.ron — this script never touches it. It rewrites only the
// generated PROSE (`lines:` and `confirms:`), so tuning the RON is never clobbered.

const PROMPT = `You are writing flavor text for a historical (early-1800s) Ohio/Mississippi
flatboat trading game. The crew hears dockside RUMORS about how the NEXT river
landing will price a good. Write template lines using EXACTLY the placeholders
{good} (a lowercase trade good like "whiskey", "hides") and {town} (a town name
like "Natchez"). No other placeholders. Period voice, terse, vivid, no quotes.

Return ONLY minified JSON with these keys, each an array of 6-8 distinct strings:
- "dear": the good will sell DEAR at {town} (scarce) — encourage loading it.
- "cheap": the good is a GLUT at {town} (worthless there) — discourage carrying it.
- "ordinary": the good fetches a MIDDLING/usual price at {town}.
- "held": payoff line, the earlier tip PROVED TRUE about {good} at {town}.
- "wind": payoff line, the earlier tip was FALSE about {good} at {town}.`;

function ronStr(s) {
  // Escape for a RON double-quoted string. Control chars (esp. a stray newline
  // from the LLM) must be escaped or they break the literal / corrupt the file.
  return (
    '"' +
    s
      .replace(/\\/g, '\\\\')
      .replace(/"/g, '\\"')
      .replace(/\n/g, '\\n')
      .replace(/\r/g, '\\r')
      .replace(/\t/g, '\\t') +
    '"'
  );
}

// Build just the generated prose sub-block (`lines:` + `confirms:`), to splice in
// place of the existing one. `sources:` is left untouched.
function proseBlock(corpus) {
  const tmpl = (arr) => arr.map((t) => `                ${ronStr(t)},`).join('\n');
  const line = (kind, arr) => `            (kind: "${kind}", templates: [\n${tmpl(arr)}\n            ]),`;
  return (
    `\n        lines: [\n${line('dear', corpus.dear)}\n${line('ordinary', corpus.ordinary)}\n${line('cheap', corpus.cheap)}\n        ],` +
    `\n        confirms: [\n${line('held', corpus.held)}\n${line('wind', corpus.wind)}\n        ],`
  );
}

async function generate() {
  const key = process.env.ANTHROPIC_API_KEY;
  if (!key) {
    console.error('ANTHROPIC_API_KEY not set. Export it (or run `just gen-rumors`) to regenerate.');
    process.exit(2);
  }
  const res = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'x-api-key': key, 'anthropic-version': '2023-06-01' },
    body: JSON.stringify({ model: MODEL, max_tokens: 2000, messages: [{ role: 'user', content: PROMPT }] }),
  });
  if (!res.ok) throw new Error(`Anthropic API ${res.status}: ${await res.text()}`);
  const data = await res.json();
  const text = data.content.map((b) => b.text ?? '').join('');
  const json = JSON.parse(text.slice(text.indexOf('{'), text.lastIndexOf('}') + 1));
  for (const k of ['dear', 'cheap', 'ordinary', 'held', 'wind']) {
    if (!Array.isArray(json[k]) || json[k].length === 0) throw new Error(`missing/empty "${k}" in LLM output`);
  }
  return json;
}

const corpus = await generate();
const ron = readFileSync(RON, 'utf8');
// Replace ONLY the generated prose: from the rumors block's `lines:` sub-field up
// to (but not including) the tuple's own `\n    ),` close. This leaves `sources:`
// and the `rumors: (` header untouched, and — by anchoring on the rumors block's
// own 4-space close rather than the file's final `)` — survives any field added
// after `rumors:`.
const blockStart = ron.indexOf('\n    rumors: (');
if (blockStart < 0) throw new Error('no `rumors:` block found in kaintuck.ron');
const proseStart = ron.indexOf('\n        lines: [', blockStart);
if (proseStart < 0) throw new Error('no `lines:` sub-field inside the rumors block');
const blockEnd = ron.indexOf('\n    ),', proseStart); // the rumors tuple's close
if (blockEnd < 0) throw new Error('could not find the end of the rumors block');
const next = ron.slice(0, proseStart) + proseBlock(corpus) + ron.slice(blockEnd);
writeFileSync(RON, next);
console.log(`Rewrote rumors prose in ${RON} (${Object.values(corpus).flat().length} lines; sources untouched).`);
console.log('Now run: cargo test -p kaintuck-engine   (validates parse + placeholders, re-pins golden hash if needed)');
