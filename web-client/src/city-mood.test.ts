import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { mapDecisionOptions } from './decision-controls.ts';

const css = readFileSync(new URL('./style.css', import.meta.url), 'utf8');
const engine = createRequire(import.meta.url)('../.engine/server.js');

// Rules that remove whole city mood faces from the map.
function hiddenMoodSelectors() {
  return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)]
    .filter(([, , body]) => /display:\s*none/.test(body))
    .map(([, selector]) => selector.trim().replace(/\s+/g, ' '))
    .filter((selector) => /\.city-map-label(?![-\w])/.test(selector));
}

test('card effects and casualty choices keep every city mood face on the map', () => {
  const game = JSON.parse(
    readFileSync(
      new URL('../../server/tests/test_games/incidents/pandemics/pandemics.outcome.json', import.meta.url),
      'utf8',
    ),
  );
  const view = JSON.parse(engine.webView(engine.stripSecret(JSON.stringify(game), 0), 0));
  // Pandemics asks for units, which marks the board as `.choosing-pieces`.
  assert.ok(mapDecisionOptions(view.decision).some((option) => option.mapTarget));
  assert.deepEqual(
    hiddenMoodSelectors().filter((selector) => !selector.includes('.collecting')),
    [],
    'Only resource collection may swap mood faces for its own badges',
  );
  // Faces stay visible but must not intercept a click aimed at a unit model.
  assert.match(css, /\.world-labels\.choosing-pieces \.city-map-label \{\s*pointer-events: none;/);
});
