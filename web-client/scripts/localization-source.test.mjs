import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { decodeRustString, rustStrings, rustTemplate } from './localization-source.mjs';

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
