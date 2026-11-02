import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { parse } from 'svelte/compiler';
import { sourceMessage, svelteStrings, decodeRustString, rustStrings, rustTemplate } from './localization-source.mjs';

const root = new URL('../', import.meta.url);
test('Rust continuation escapes match Rust whitespace and Unicode semantics', () => {
  assert.equal(decodeRustString('First \\\n    second \\\r\n\t third'), 'First second third');
  assert.equal(decodeRustString(String.raw`A\nB\t\u{1F_600}\x21\"\\`), 'A\nB\t😀!"\\');
  assert.equal(rustTemplate('Gain {amount} at {position:?}; {{literal}} {} {0:.2}'), 'Gain {p0} at {p1}; {literal} {p2} {p3}');
});
test('Rust lexer ignores comments and character literals, preserving raw strings', () => {
  const input = `// "Not a rule"
/* "Nor this" /* nested */ */
let quote = '"'; let lifetime: &'a str = "A rule";
let raw = r##"An unescaped "quote" and \\n"##;
let continued = "First \\
    second";`;
  assert.deepEqual(rustStrings(input).map(({ value }) => value), [
    'A rule', 'An unescaped "quote" and \\n', 'First second',
  ]);
});
test('visible Svelte punctuation-prefixed prose is inventoried without admitting selectors', () => {
  const fixture = `
    <script>
      const selectors = ['.panel button', '#game button', ':scope > button', '[data-action]'];
    </script>
    <p class="Ignore class text">
      . If they pass, other players may take it in turn order for <strong>2</strong>.
    </p>
    <p>: Player {name}</p>
    <p># Round summary</p>
    <p title="[Optional] Help">[Optional] Choose a card</p>
    <style>.panel button { --caption: "Ignore CSS text"; }</style>
  `;
  // Use fresh parsed markup, not the seeded English catalog: these candidates
  // must reach the same inventory that --check compares against en.json.
  const candidates = [...svelteStrings(parse(fixture, { modern: true }))]
    .map(({ value, visibleText }) => sourceMessage(value, { visibleText }))
    .filter((text) => text !== null);
  assert.deepEqual(new Set(candidates), new Set([
    '. If they pass, other players may take it in turn order for',
    ': Player {p0}',
    ': Player',
    '# Round summary',
    '[Optional] Help',
    '[Optional] Choose a card',
  ]));
  // Other sources still use technical filtering, and unrelated heuristics stay
  // intact even when a candidate comes from visible markup.
  for (const value of ['.panel button', '#game button', ':scope > button', '[data-action]'])
    assert.equal(sourceMessage(value), null, value);
  for (const value of ['button[data-panel-close]', 'action-panel floating-panel', 'Build {p0}{p1}'])
    assert.equal(sourceMessage(value, { visibleText: true }), null, value);
});
test('extracts current continued engine rules and omits test/technical noise', () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'clash-locales-'));
  try {
    const output = path.join(directory, 'en.json');
    const locations = path.join(directory, 'locations.json');
    const run = spawnSync(process.execPath, ['scripts/extract-localization.mjs', `--source-output=${output}`, `--locations=${locations}`], { cwd: root, encoding: 'utf8' });
    assert.equal(run.status, 0, run.stderr);
    const inventory = JSON.parse(fs.readFileSync(locations, 'utf8'));
    for (const prefix of [
      'Look at another player\'s hand of Wonder, Action, and Objective cards.',
      'You may move any number of Barbarian armies 1 space each, which may start combat.',
      'A: When drawing a wonder: Take {p0} from anywhere in the game',
    ]) assert.ok(Object.keys(inventory).some((key) => key.startsWith(prefix)), prefix);
    for (const key of ['Build {p0} at {p1}', 'Free with {p0} · once per turn', 'city size {p0}, Fortress', 'Recruit {p0} units & action card']) assert.ok(inventory[key], key);
    for (const key of ['action-panel floating-panel early-choices', 'current={p0}, available={p1}, conversions={p2}, exact={p3}, skip={p4}', 'crate::timing::deserialize_counters', 'button[data-panel-close]', 'Build {p0}{p1}']) assert.equal(inventory[key], undefined, key);
    assert.ok(Object.values(inventory).flat().every((file) => !/\.test\.ts|_tests\.rs/.test(file)));
    // Check mode must not write even if output flags are accidentally supplied.
    const before = fs.readFileSync(output, 'utf8');
    fs.writeFileSync(locations, 'sentinel');
    const check = spawnSync(process.execPath, ['scripts/extract-localization.mjs', '--check', `--source-output=${output}`, `--locations=${locations}`], { cwd: root, encoding: 'utf8' });
    assert.ok([0, 1].includes(check.status), check.stderr);
    assert.equal(fs.readFileSync(output, 'utf8'), before);
    assert.equal(fs.readFileSync(locations, 'utf8'), 'sentinel');
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
