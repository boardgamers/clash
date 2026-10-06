import fs from 'node:fs';
import path from 'node:path';
import ts from 'typescript';
import { parse } from 'svelte/compiler';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('..', import.meta.url));
const source = JSON.parse(fs.readFileSync(path.join(root, '../client/js/src/localization/en.json'), 'utf8'));
const existingSource = path.join(root, 'src/localization/en.json');
if (fs.existsSync(existingSource)) Object.assign(source, JSON.parse(fs.readFileSync(existingSource, 'utf8')));
const locations = new Map();
function add(value, file) {
  const text = value.replace(/\s+/g, ' ').trim();
  if (
    !/[A-Za-z]/.test(text) ||
    text.length < 2 ||
    /^[-a-z0-9:/]+$/.test(text) ||
    /^[#.:\[]/.test(text) ||
    /^<[a-z]/.test(text) ||
    /[A-Za-z]__[A-Za-z]/.test(text) ||
    /=>|\\[bpdw]|\.\/(?:src|assets)|https?:|\.svg|\.wasm|rgba?\(|gradient\(|var\(|\d(?:px|rem)\b/.test(text)
  )
    return;
  source[text] = text;
  if (!locations.has(text)) locations.set(text, []);
  locations.get(text).push(file);
}
function javascript(node, file) {
  if (ts.isStringLiteral(node)) add(node.text, file);
  if (ts.isTemplateExpression(node))
    add(node.head.text + node.templateSpans.map((span, i) => `{p${i}}${span.literal.text}`).join(''), file);
  ts.forEachChild(node, (child) => javascript(child, file));
}
function svelte(node, file) {
  if (!node || typeof node !== 'object') return;
  if (
    node.type === 'Fragment' &&
    node.nodes?.some((n) => n.type === 'ExpressionTag') &&
    node.nodes.every((n) => ['Text', 'ExpressionTag', 'Comment'].includes(n.type))
  ) {
    let index = 0;
    add(
      node.nodes
        .map((n) => (n.type === 'Text' ? n.data : n.type === 'ExpressionTag' ? `{p${index++}}` : ''))
        .join(''),
      file,
    );
  }
  if (node.type === 'Text') add(node.data, file);
  if (node.type === 'Literal' && typeof node.value === 'string') add(node.value, file);
  if (node.type === 'TemplateLiteral')
    add(
      node.quasis.map((q, i) => q.value.cooked + (i < node.expressions.length ? `{p${i}}` : '')).join(''),
      file,
    );
  for (const [key, value] of Object.entries(node)) {
    if (['css', 'start', 'end', 'loc'].includes(key)) continue;
    if (Array.isArray(value)) value.forEach((v) => svelte(v, file));
    else if (value && typeof value === 'object') svelte(value, file);
  }
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
    else svelte(parse(content, { modern: true }), file);
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
    if (item.isDirectory()) rust(file);
    else if (item.name.endsWith('.rs')) {
      const content = fs.readFileSync(file, 'utf8');
      for (const match of content.matchAll(/"((?:[^"\\]|\\.)*)"/g)) {
        let value;
        try {
          value = JSON.parse(`"${match[1]}"`);
        } catch {
          continue;
        }
        let index = 0;
        value = value.replace(/\{(?:[a-zA-Z_][a-zA-Z_0-9]*|(?::[^}]*)?)\}/g, () => `{p${index++}}`);
        add(value, path.relative(root, file));
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
fs.writeFileSync(sourceOutput, JSON.stringify(sorted, null, 2) + '\n');
const out = process.argv.find((arg) => arg.startsWith('--locations='))?.slice('--locations='.length);
if (out) fs.writeFileSync(out, JSON.stringify(Object.fromEntries(locations), null, 2) + '\n');
console.log(`${Object.keys(source).length} source strings extracted`);
