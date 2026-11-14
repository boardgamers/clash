import fs from 'node:fs';
import { sourceMessage, svelteStrings, rustStrings, rustTemplate } from './localization-source.mjs';
import path from 'node:path';
import ts from 'typescript';
import { parse } from 'svelte/compiler';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('..', import.meta.url));
const source = JSON.parse(fs.readFileSync(path.join(root, '../client/js/src/localization/en.json'), 'utf8'));
const existingSource = path.join(root, 'src/localization/en.json');
if (fs.existsSync(existingSource)) Object.assign(source, JSON.parse(fs.readFileSync(existingSource, 'utf8')));
const locations = new Map();
function add(value, file, options) {
  const text = sourceMessage(value, options);
  if (text === null) return;
  source[text] = text;
  if (!locations.has(text)) locations.set(text, []);
  locations.get(text).push(file);
}
function javascript(node, file) {
  if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) return;
  if ((ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) && !(ts.isPropertyAssignment(node.parent) && node.parent.name === node)) add(node.text, file);
  if (ts.isTemplateExpression(node))
    add(node.head.text + node.templateSpans.map((span, i) => `{p${i}}${span.literal.text}`).join(''), file);
  ts.forEachChild(node, (child) => javascript(child, file));
}
function frontend(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (entry.name !== 'localization') frontend(path.join(directory, entry.name));
      continue;
    }
    const name = entry.name;
    if (/\.test\.ts$/.test(name) || !/\.(ts|svelte)$/.test(name)) continue;
    const absolute = path.join(directory, name);
    const content = fs.readFileSync(absolute, 'utf8');
    const file = path.relative(root, absolute);
    if (name.endsWith('.ts'))
      javascript(ts.createSourceFile(name, content, ts.ScriptTarget.Latest, true), file);
    else {
      for (const { value, visibleText } of svelteStrings(parse(content, { modern: true })))
        add(value, file, { visibleText });
    }
  }
}
frontend(path.join(root, 'src'));
const guide = path.join(root, 'node_modules/@boardgamers/protocol/dist/tutorial-dom.js');
javascript(
  ts.createSourceFile(guide, fs.readFileSync(guide, 'utf8'), ts.ScriptTarget.Latest, true),
  'protocol/tutorial/dom',
);
function rust(directory) {
  for (const item of fs.readdirSync(directory, { withFileTypes: true })) {
    const file = path.join(directory, item.name);
    // BGS consumes this catalog as platform metadata, not viewer-localized prose.
    // It has its own revisioned names/descriptions and is never rendered by this viewer.
    if (item.name === 'achievements.rs') continue;
    if (item.isDirectory()) rust(file);
    else if (item.name.endsWith('.rs') && !/(?:^|_)tests?\.rs$/.test(item.name) && item.name !== 'timing.rs') {
      const content = fs.readFileSync(file, 'utf8');
      for (const { value, start } of rustStrings(content)) {
        // Ignore test modules and diagnostics: these are not player-facing rules.
        if (/#\[cfg\(test\)\]/.test(content.slice(0, start))) continue;
        const context = content.slice(Math.max(0, start - 120), start);
        if (/(?:panic!|unreachable!|assert(?:_eq|_ne)?!|debug_assert(?:_eq|_ne)?!|expect)\s*\([^;]*$/.test(context)) continue;
        add(rustTemplate(value), path.relative(root, file));
      }
    }
  }
}
rust(path.join(root, '../server/src'));
// Atomic words occur inside pluralized/interpolated UI captions.
for (const word of 'action actions advance advances city cities card cards wonder wonders unit units point points event events marker markers activation activations leader leaders round rounds age ages mood culture building buildings of and or for to from at with without remaining left'.split(
  ' ',
))
  source[word] = word;
for (const phrase of ['event marker', 'event markers', 'Once per turn', 'City size']) source[phrase] = phrase;
const sorted = Object.fromEntries(Object.entries(source).sort(([a], [b]) => a.localeCompare(b)));
const sourceOutput =
  process.argv.find((arg) => arg.startsWith('--source-output='))?.slice('--source-output='.length) ??
  path.join(root, 'src/localization/en.json');
if (process.argv.includes('--check')) {
  const current = JSON.parse(fs.readFileSync(existingSource, 'utf8'));
  const missing = [...locations.keys()].filter((key) => !Object.hasOwn(current, key));
  for (const key of missing) console.error(`${locations.get(key).join(', ')}: ${key}`);
  if (missing.length) {
    console.error(`${missing.length} player-facing source strings missing from en.json`);
    process.exitCode = 1;
  } else console.log('Current UI/engine inventory is present in en.json');
} else fs.writeFileSync(sourceOutput, JSON.stringify(sorted, null, 2) + '\n');
const out = process.argv.find((arg) => arg.startsWith('--locations='))?.slice('--locations='.length);
if (out && !process.argv.includes('--check')) fs.writeFileSync(out, JSON.stringify(Object.fromEntries(locations), null, 2) + '\n');
console.log(`${Object.keys(source).length} source strings extracted`);
