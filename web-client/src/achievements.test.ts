import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { Engine, AchievementManifest, SeatAchievements } from '@boardgamers/protocol/engine';
import { achievementManifestSchema, achievementResultsSchema } from '@boardgamers/protocol/achievements';

const require = createRequire(import.meta.url);
// Check the generated WASM API, not a parallel JS achievement implementation.
const engine: typeof import('../.bridge/server.js') = require('../.engine/server.js');
const hooks = {
  achievementDefinitions: (): AchievementManifest => engine.achievementDefinitions(),
  achievements: (data: string): SeatAchievements[] => engine.achievements(data),
} satisfies Pick<Engine<string>, 'achievementDefinitions' | 'achievements'>;
const fixture = readFileSync(
  new URL('../../server/tests/test_games/status_phase/end_game.json', import.meta.url),
  'utf8',
);
const finish = () => engine.tryMove(fixture, JSON.stringify({ Playing: 'EndTurn' }), 0);

test('WASM achievement catalog follows official protocol 0.11.0', () => {
  const manifest = achievementManifestSchema.parse(hooks.achievementDefinitions());
  assert.equal(manifest.revision, 3);
  assert.equal(manifest.achievements.length, 41);
  assert.ok(!manifest.achievements.some((a) => a.id === 'four-cities'));
  assert.ok(manifest.achievements.some((a) => a.id === 'seven-cities'));
  assert.ok(manifest.achievements.some((a) => a.id === 'complete-builder'));
  assert.equal(manifest.achievements.find((a) => a.id === 'complete-standard')?.name, 'Complete Standard');
  assert.equal(manifest.achievements.find((a) => a.id === 'win-standard')?.name, 'Win Standard');
  assert.ok(manifest.achievements.every((a) => !/\bNormal\b/.test(`${a.name} ${a.description}`)));
  assert.deepEqual(hooks.achievementDefinitions(), manifest);
  const art = JSON.parse(
    readFileSync(new URL('../release/achievement-assets.json', import.meta.url), 'utf8'),
  );
  const nonfactionArt = JSON.parse(
    readFileSync(new URL('../release/achievement-assets-nonfaction.json', import.meta.url), 'utf8'),
  );
  assert.equal(nonfactionArt.revision, manifest.revision);
  assert.equal(art.revision, 2, 'revision 3 preserves the existing faction artwork');
  assert.deepEqual(
    manifest.achievements
      .filter((a) => a.icon?.type === 'asset')
      .map((a) => (a.icon!.type === 'asset' ? a.icon!.key : ''))
      .sort(),
    [...art.assets, ...nonfactionArt.assets].map((a: { key: string }) => a.key).sort(),
  );
  for (const length of ['shorter', 'standard', 'roll-to-end', 'wonderful-ending', 'epic']) {
    for (const verb of ['complete', 'win'])
      assert.ok(manifest.achievements.some((a) => a.id === `${verb}-${length}`));
  }
});

test('actual completed fixture produces valid, deterministic, immutable seat achievements', () => {
  assert.deepEqual(hooks.achievements(fixture), []);
  const finished = finish();
  assert.equal(engine.ended(finished), true);
  const before = JSON.parse(finished);
  const manifest = hooks.achievementDefinitions();
  const results = achievementResultsSchema(manifest, 2).parse(hooks.achievements(finished));
  assert.deepEqual(
    results.map((r) => r.seat),
    [0, 1],
  );
  const ranks = engine.rankings(finished) as number[];
  for (const result of results) {
    assert.ok(result.earned.includes('complete-standard'));
    const outcome =
      ranks[result.seat] === 1 ? (ranks.filter((r) => r === 1).length > 1 ? 'draw' : 'win') : 'loss';
    assert.equal(result.outcome, outcome);
    assert.equal(result.earned.includes('win-standard'), outcome === 'win');
  }
  assert.deepEqual(hooks.achievements(finished), results);
  assert.deepEqual(hooks.achievements(JSON.stringify(before)), results);
  assert.deepEqual(JSON.parse(finished), before);
});

test('WASM preserves inclusive score boundaries and awards feats to losing seats', () => {
  for (const [score, thirty, forty] of [
    [29.5, false, false],
    [30, true, false],
    [39.5, true, false],
    [40, true, true],
  ] as const) {
    const finished = finish();
    const data = JSON.parse(finished);
    const scores = engine.scores(finished) as number[];
    for (const seat of [0, 1]) {
      data.players[seat].special_victory_points ??= [];
      data.players[seat].special_victory_points.push({
        points: (seat === 0 ? score : 100) - scores[seat],
        origin: { Ability: 'achievement boundary test' },
        attribution: 'Events',
      });
    }
    const result = hooks.achievements(JSON.stringify(data)).find((r) => r.seat === 0)!;
    assert.equal(result.outcome, 'loss');
    assert.equal(result.earned.includes('score-30'), thirty);
    assert.equal(result.earned.includes('score-40'), forty);
    assert.ok(result.earned.includes('complete-standard'));
  }
});

test('all length options survive the serialized WASM boundary', () => {
  for (const [option, suffix] of [
    ['Shorter', 'shorter'],
    ['Standard', 'standard'],
    ['RollToEnd', 'roll-to-end'],
    ['WonderfulEnding', 'wonderful-ending'],
    ['Epic', 'epic'],
  ]) {
    const data = JSON.parse(finish());
    data.options = { ...data.options, length: option };
    const results = hooks.achievements(JSON.stringify(data));
    assert.equal(results.length, 2);
    for (const result of results) {
      assert.deepEqual(
        result.earned.filter((id) => id.startsWith('complete-')),
        [`complete-${suffix}`],
      );
    }
  }
});

const civilizations = [
  'Rome',
  'Greece',
  'China',
  'Vikings',
  'Babylonia',
  'India',
  'Egypt',
  'Phoenicia',
  'Maya',
  'Persia',
  'Japan',
  'Huns',
  'Celts',
  'Aztecs',
  'Carthage',
];

test('all actual civilizations have distinct faction-win badges and durable named asset keys', () => {
  const manifest = hooks.achievementDefinitions();
  const factionIds = civilizations.map((name) => `win-${name.toLowerCase()}`);
  for (const civilization of civilizations) {
    const id = `win-${civilization.toLowerCase()}`;
    const definition = manifest.achievements.find((a) => a.id === id)!;
    assert.ok(definition, civilization);
    assert.deepEqual(definition.icon, {
      type: 'asset',
      key: `achievement-faction-${civilization.toLowerCase()}`,
    });
    const data = JSON.parse(finish());
    data.players[0].civilization = civilization;
    data.players[0].special_victory_points = [
      { points: 100, origin: { Ability: 'faction fixture' }, attribution: 'Events' },
    ];
    const results = achievementResultsSchema(manifest, 2).parse(hooks.achievements(JSON.stringify(data)));
    assert.equal(results[0].outcome, 'win');
    assert.deepEqual(
      results[0].earned.filter((id) => factionIds.includes(id)),
      [id],
    );
    assert.equal(results[1].outcome, 'loss');
    assert.deepEqual(
      results[1].earned.filter((id) => factionIds.includes(id)),
      [],
    );
    assert.deepEqual(hooks.achievements(engine.stripSecret(JSON.stringify(data), 0)), []);
    data.state = 'Playing';
    assert.deepEqual(hooks.achievements(JSON.stringify(data)), []);
  }
});

test('seven-city badge uses final city count: six fails, seven passes, no retired ID', () => {
  for (const count of [6, 7]) {
    const data = JSON.parse(finish());
    const city = data.players[0].cities[0];
    data.players[0].cities = Array.from({ length: count }, (_, i) => ({
      ...structuredClone(city),
      position: `A${i + 1}`,
    }));
    const results = hooks.achievements(JSON.stringify(data));
    assert.equal(results[0].earned.includes('seven-cities'), count === 7);
    assert.ok(results.every((r) => !r.earned.includes('four-cities')));
  }
});

test('Builder reaches real completion and awards both winner and loser, never ongoing or filtered', () => {
  const data = JSON.parse(fixture);
  data.options = { ...data.options, variant: 'Builder' };
  data.players[0].special_victory_points = [
    { points: 100, origin: { Ability: 'builder fixture' }, attribution: 'Events' },
  ];
  const raw = JSON.stringify(data);
  assert.deepEqual(hooks.achievements(raw), []);
  const completed = engine.tryMove(raw, JSON.stringify({ Playing: 'EndTurn' }), 0);
  assert.equal(engine.ended(completed), true);
  const results = hooks.achievements(completed);
  assert.equal(results.length, 2);
  assert.ok(results.some((r) => r.outcome === 'loss'));
  for (const result of results) {
    assert.ok(result.earned.includes('complete-builder'));
    assert.ok(result.earned.includes('complete-standard'));
  }
  for (const seat of [undefined, 0, 1])
    assert.deepEqual(hooks.achievements(engine.stripSecret(completed, seat)), []);
  assert.ok(hooks.achievements(finish()).every((r) => !r.earned.includes('complete-builder')));
});

const wonders = [
  ['Colosseum', 'great-arena'],
  ['Pyramids', 'great-pyramids'],
  ['GreatGardens', 'great-gardens'],
  ['GreatLibrary', 'great-library'],
  ['GreatLighthouse', 'great-lighthouse'],
  ['GreatMausoleum', 'great-mausoleum'],
  ['GreatStatue', 'great-statue'],
  ['GreatWall', 'great-wall'],
];

test('each real wonder awards only its surviving builder record, even to a losing original builder', () => {
  const manifest = hooks.achievementDefinitions();
  assert.deepEqual(
    manifest.achievements
      .filter((a) => a.id.startsWith('built-'))
      .map((a) => a.id)
      .sort(),
    wonders.map(([, slug]) => `built-${slug}`).sort(),
  );
  for (const [wonder, slug] of wonders) {
    const data = JSON.parse(finish());
    for (const player of data.players) {
      player.wonders_built = [];
      for (const city of player.cities ?? []) if (city.city_pieces) city.city_pieces.wonders = [];
    }
    assert.ok(hooks.achievements(JSON.stringify(data)).every((r) => !r.earned.includes(`built-${slug}`)));
    // The wonder survives in the opponent's city after capture; only seat 0 built it.
    data.players[0].wonders_built = [wonder];
    data.players[1].cities[0].city_pieces ??= {};
    data.players[1].cities[0].city_pieces.wonders = [wonder];
    data.players[1].special_victory_points = [
      { points: 100, origin: { Ability: 'wonder fixture' }, attribution: 'Events' },
    ];
    const raw = JSON.stringify(data);
    const results = achievementResultsSchema(manifest, 2).parse(hooks.achievements(raw));
    assert.equal(results[0].outcome, 'loss');
    assert.deepEqual(
      results[0].earned.filter((id) => id.startsWith('built-')),
      [`built-${slug}`],
    );
    assert.ok(!results[0].earned.includes('two-wonders-built'));
    assert.deepEqual(
      results[1].earned.filter((id) => id.startsWith('built-')),
      [],
    );
    assert.deepEqual(hooks.achievements(JSON.stringify(JSON.parse(raw))), results);
    assert.deepEqual(hooks.achievements(engine.stripSecret(raw, 0)), []);
    // This is the persisted shape after destruction removes both piece and record.
    data.players[0].wonders_built = [];
    data.players[1].cities[0].city_pieces.wonders = [];
    assert.ok(hooks.achievements(JSON.stringify(data)).every((r) => !r.earned.includes(`built-${slug}`)));
    data.state = 'Playing';
    data.players[0].wonders_built = [wonder];
    assert.deepEqual(hooks.achievements(JSON.stringify(data)), []);
  }
});

test('ongoing analysis branches and their filtered views do not earn badges', async () => {
  const raw = await engine.init(2, [], { variant: 'Builder' }, 'achievement-analysis', {});
  assert.equal(engine.canLaunchAnalysisMode(raw), true);
  const analysis = engine.createAnalysisScenario(raw, { player: 0, seed: 'achievement-analysis-branch' });
  assert.equal(engine.ended(analysis), false);
  assert.deepEqual(hooks.achievements(analysis), []);
  assert.deepEqual(hooks.achievements(engine.stripSecret(analysis, 0)), []);
  // Completed practice/analysis provenance remains a platform eligibility check:
  // the engine save has no persistent provenance field.
});

test('filtered snapshots and all-withdrawn completions cannot award achievements', () => {
  const finished = finish();
  for (const seat of [undefined, 0, 1]) {
    assert.deepEqual(hooks.achievements(engine.stripSecret(finished, seat)), []);
  }
  const dropped = JSON.parse(finished);
  dropped.dropped_players = [0, 1];
  assert.deepEqual(hooks.achievements(JSON.stringify(dropped)), []);
});
