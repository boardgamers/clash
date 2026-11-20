import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import type { Engine, AchievementManifest, SeatAchievements } from '@boardgamers/protocol/engine';
import { achievementManifestSchema, achievementResultsSchema } from '@boardgamers/protocol/achievements';

const require = createRequire(import.meta.url);
// Check the generated WASM API, not a parallel JS achievement implementation.
const engine: typeof import('../.bridge/server.js') = require(
  process.env.CLASH_TEST_ENGINE ?? '../.engine/server.js',
);
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
  assert.equal(manifest.revision, 8);
  assert.equal(manifest.achievements.length, 46);
  assert.deepEqual(
    manifest.achievements.slice(10, 22).map((a) => a.id),
    [
      'score-30',
      'score-40',
      'score-50',
      'score-60',
      'seven-cities',
      'twenty-advances',
      'thirty-advances',
      'three-objectives',
      'five-objectives',
      'seven-objectives',
      'two-wonders-built',
      'captured-leader',
    ],
  );
  assert.deepEqual(
    manifest.achievements.find((a) => a.id === 'score-50'),
    {
      id: 'score-50',
      name: '50 Points',
      description: 'Finish a game with at least 50 victory points in the final state.',
      icon: { type: 'asset', key: 'achievement-score-50' },
    },
  );
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
  assert.equal(nonfactionArt.revision, 4, 'revision 8 adds dedicated score artwork');
  assert.equal(art.revision, 2, 'revision 7 preserves the existing faction artwork');
  assert.deepEqual(
    [
      ...new Set(
        manifest.achievements
          .filter((a) => a.icon?.type === 'asset')
          .map((a) => (a.icon!.type === 'asset' ? a.icon!.key : '')),
      ),
    ].sort(),
    [...art.assets, ...nonfactionArt.assets].map((a: { key: string }) => a.key).sort(),
  );
  for (const length of ['shorter', 'standard', 'roll-to-end', 'wonderful-ending', 'epic']) {
    for (const verb of ['complete', 'win'])
      assert.ok(manifest.achievements.some((a) => a.id === `${verb}-${length}`));
  }
});

test('catalog preserves previous definitions except wonder names, score artwork and revision 6 additions/order', () => {
  // Captured from the generated ba8ee4e2 / engine 0.4.157 catalog before rebuilding.
  const previous: AchievementManifest = JSON.parse(
    readFileSync(new URL('./fixtures/achievements-revision-5.json', import.meta.url), 'utf8'),
  );
  const manifest = hooks.achievementDefinitions();
  const previousById = new Map(previous.achievements.map((a) => [a.id, a]));
  // Revision 7 changes only these eight names; keep the revision-5 fixture historical.
  const wonderNames = new Map([
    ['built-great-arena', 'Great Arena'],
    ['built-great-pyramids', 'Great Pyramids'],
    ['built-great-gardens', 'Great Gardens'],
    ['built-great-library', 'Great Library'],
    ['built-great-lighthouse', 'Great Lighthouse'],
    ['built-great-mausoleum', 'Great Mausoleum'],
    ['built-great-statue', 'Great Statue'],
    ['built-great-wall', 'Great Wall'],
  ]);
  const expectedIds = [
    ...previous.achievements.slice(0, 10).map((a) => a.id),
    'score-30',
    'score-40',
    'score-50',
    'score-60',
    'seven-cities',
    'twenty-advances',
    'thirty-advances',
    'three-objectives',
    'five-objectives',
    'seven-objectives',
    'two-wonders-built',
    'captured-leader',
    ...previous.achievements.slice(21).map((a) => a.id),
  ];
  assert.deepEqual(
    manifest.achievements,
    expectedIds.map((id) =>
      id === 'score-50'
        ? {
            id,
            name: '50 Points',
            description: 'Finish a game with at least 50 victory points in the final state.',
            icon: { type: 'asset', key: 'achievement-score-50' },
          }
        : id === 'score-60'
          ? { ...previousById.get(id), icon: { type: 'asset', key: 'achievement-score-60' } }
          : wonderNames.has(id)
            ? { ...previousById.get(id), name: wonderNames.get(id) }
            : previousById.get(id),
    ),
  );
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
  for (const [score, thirty, forty, fifty] of [
    [29.5, false, false, false],
    [30, true, false, false],
    [39.5, true, false, false],
    [40, true, true, false],
    [49, true, true, false],
    [50, true, true, true],
    [51, true, true, true],
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
    assert.equal(result.earned.includes('score-50'), fifty);
    assert.equal((engine.scores(JSON.stringify(data)) as number[])[0], score);
    assert.deepEqual(hooks.achievements(engine.stripSecret(JSON.stringify(data), 0)), []);
    data.dropped_players = [0];
    assert.ok(hooks.achievements(JSON.stringify(data)).every((r) => r.seat !== 0));
    data.dropped_players = [];
    data.state = 'Playing';
    assert.deepEqual(hooks.achievements(JSON.stringify(data)), []);
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

test('five objectives uses completed records, with inclusive boundaries and unchanged eligibility', () => {
  const manifest = hooks.achievementDefinitions();
  assert.deepEqual(
    manifest.achievements.find((a) => a.id === 'five-objectives'),
    {
      id: 'five-objectives',
      name: 'Five Objectives',
      description: 'Complete 5 objectives in a single game.',
      icon: { type: 'asset', key: 'achievement-three-objectives' },
    },
  );
  for (const count of [4, 5, 6]) {
    const data = JSON.parse(finish());
    data.players[0].completed_objectives = Array.from({ length: count }, (_, card) => ({
      card,
      name: `test ${card}`,
    }));
    data.players[1].completed_objectives = [];
    data.players[1].special_victory_points = [
      { points: 100, origin: { Ability: 'objective fixture' }, attribution: 'Events' },
    ];
    const raw = JSON.stringify(data);
    const results = achievementResultsSchema(manifest, 2).parse(hooks.achievements(raw));
    assert.equal(results[0].outcome, 'loss');
    assert.equal(results[0].earned.includes('five-objectives'), count >= 5);
    assert.ok(results[0].earned.includes('three-objectives'));
    assert.ok(!results[1].earned.includes('five-objectives'));
    assert.deepEqual(hooks.achievements(raw), results);
    assert.deepEqual(hooks.achievements(engine.stripSecret(raw, 0)), []);
    data.dropped_players = [0];
    assert.ok(hooks.achievements(JSON.stringify(data)).every((r) => r.seat !== 0));
    data.dropped_players = [];
    data.state = 'Playing';
    assert.deepEqual(hooks.achievements(JSON.stringify(data)), []);
  }
});

const regularAdvances = [
  'Farming',
  'Storage',
  'Irrigation',
  'Husbandry',
  'Mining',
  'Engineering',
  'Sanitation',
  'Roads',
  'Fishing',
  'Navigation',
  'WarShips',
  'Cartography',
  'Writing',
  'PublicEducation',
  'FreeEducation',
  'Philosophy',
  'Tactics',
  'Siegecraft',
  'SteelWeapons',
  'Draft',
  'Myths',
  'Rituals',
  'Priesthood',
  'StateReligion',
  'Bartering',
  'TradeRoutes',
  'Taxes',
  'Currency',
  'Arts',
  'Sports',
  'Monuments',
];

test('revision 5 milestones use inclusive canonical totals and unchanged eligibility', () => {
  const manifest = hooks.achievementDefinitions();
  for (const [id, name, description, key] of [
    [
      'score-60',
      '60 Points',
      'Finish a game with at least 60 victory points in the final state.',
      'score-60',
    ],
    [
      'seven-objectives',
      'Seven Objectives',
      'Finish a game with at least 7 completed objectives in the final state.',
      'three-objectives',
    ],
    [
      'thirty-advances',
      'Thirty Advances',
      'Finish a game with at least 30 regular advances in the final state; special advances do not count.',
      'twenty-advances',
    ],
  ]) {
    assert.deepEqual(
      manifest.achievements.find((a) => a.id === id),
      {
        id,
        name,
        description,
        icon: { type: 'asset', key: `achievement-${key}` },
      },
    );
  }
  for (const offset of [0, 1, 2]) {
    const data = JSON.parse(finish());
    const p = data.players[0];
    // Rome's special advances are derived on load, not counted as regular research.
    p.civilization = 'Rome';
    p.advances = regularAdvances.slice(0, 29 + offset);
    p.completed_objectives = Array.from({ length: 6 + offset }, (_, card) => ({
      card,
      name: `test ${card}`,
    }));
    p.objective_cards = [1, 2, 3];
    p.special_victory_points = [
      { points: 10, origin: { Ability: 'objective bonus' }, attribution: 'Objectives' },
    ];
    data.players[1].completed_objectives = [];
    const scores = engine.scores(JSON.stringify(data)) as number[];
    const target = [59, 60, 61][offset];
    for (const seat of [0, 1]) {
      data.players[seat].special_victory_points ??= [];
      data.players[seat].special_victory_points.push({
        points: (seat === 0 ? target : 100) - scores[seat],
        origin: { Ability: 'revision 5 boundary' },
        attribution: 'Events',
      });
    }
    const raw = JSON.stringify(data);
    assert.equal((engine.scores(raw) as number[])[0], target);
    const results = achievementResultsSchema(manifest, 2).parse(hooks.achievements(raw));
    assert.equal(results[0].outcome, 'loss');
    for (const id of ['score-60', 'seven-objectives', 'thirty-advances'])
      assert.equal(results[0].earned.includes(id), offset >= 1, id);
    assert.ok(!results[1].earned.includes('seven-objectives'));
    assert.ok(!results[1].earned.includes('thirty-advances'));
    assert.deepEqual(hooks.achievements(JSON.stringify(JSON.parse(raw))), results);
    assert.equal(JSON.stringify(data), raw);
    for (const seat of [undefined, 0, 1])
      assert.deepEqual(hooks.achievements(engine.stripSecret(raw, seat)), []);
    data.dropped_players = [0];
    assert.ok(hooks.achievements(JSON.stringify(data)).every((r) => r.seat !== 0));
    data.dropped_players = [];
    data.state = 'Playing';
    assert.deepEqual(hooks.achievements(JSON.stringify(data)), []);
  }
});

test('seven objectives combines real instant and status-phase completion records', () => {
  const completed = (path: string) =>
    JSON.parse(
      readFileSync(new URL(`../../server/tests/test_games/objective_cards/${path}`, import.meta.url), 'utf8'),
    ).players[0].completed_objectives;
  const data = JSON.parse(finish());
  data.players[0].completed_objectives = completed('status_phase/large_civ.outcome8.json').slice(0, 6);
  assert.ok(!hooks.achievements(JSON.stringify(data))[0].earned.includes('seven-objectives'));
  data.players[0].completed_objectives.push(...completed('instant/warmonger.outcome4.json'));
  assert.equal(data.players[0].completed_objectives.length, 7);
  assert.ok(hooks.achievements(JSON.stringify(data))[0].earned.includes('seven-objectives'));
});
