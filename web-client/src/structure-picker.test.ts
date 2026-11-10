import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { compile } from 'svelte/compiler';

// Render the actual picker, including its accessible (text-derived) button names.
const bundle = await build({
  stdin: {
    contents: `import Picker from './StructurePicker.svelte';
      import { render } from 'svelte/server';
      export const html = (choices) => render(Picker, { props: {
        choices, selected: [], position: 'A1', limit: 3,
        onPosition() {}, onSelect() {}, onHighlight() {}
      }}).body;`,
    resolveDir: fileURLToPath(new URL('.', import.meta.url)),
  },
  bundle: true,
  write: false,
  platform: 'node',
  format: 'esm',
  plugins: [
    {
      name: 'svelte-test',
      setup(builder) {
        builder.onLoad({ filter: /\.svelte$/ }, ({ path }) => ({
          contents: compile(readFileSync(path, 'utf8'), { filename: path, generate: 'server' }).js.code,
          loader: 'js',
        }));
      },
    },
  ],
});
const { html } = await import(
  `data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString('base64')}`
);

test('structure choices name only foreign owners relative to the city, not the viewer', () => {
  const base = { id: 0, position: 'A1', structure: { Building: 'Fortress' }, cityOwner: 1 };
  const foreign = html([{ ...base, owner: { player: 0, civilization: 'Rome' } }]);
  assert.match(foreign, /structure-owner/);
  assert.ok(!foreign.includes('Owner:'));
  assert.match(foreign, /Rome/);
  assert.ok(
    !foreign.includes('aria-label="Fortress"'),
    'owner must not be hidden by a shorter accessible name',
  );
  for (const choice of [
    { ...base, owner: { player: 1, civilization: 'Greece' } },
    { ...base, structure: 'CityCenter', owner: { player: 1, civilization: 'Greece' } },
    { ...base, structure: { Wonder: 'GreatGardens' }, owner: { player: 1, civilization: 'Greece' } },
    { ...base, owner: null },
    { id: 0, position: 'A1', structure: { Building: 'Temple' } },
  ])
    assert.ok(!html([choice]).includes('structure-owner'), JSON.stringify(choice));
});
