import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { chromium } from 'playwright';
import { inspectEngine } from '@boardgamers/protocol/engine';
import { ids, svg } from './nonfaction-art.mjs';
import { validateOutputs, outputDir } from './nonfaction-assets.mjs';
import { root } from './achievement-asset-inputs.mjs';
// Last delivered release: revision 5, 45 definitions.
const baseline = 'ba8ee4e2';
test('existing evaluation and faction assets are unchanged apart from score-50 and tier ordering', async () => {
  const file = 'server/src/achievements.rs';
  const old = execFileSync('git', ['show', `${baseline}:${file}`], { cwd: root, encoding: 'utf8' });
  const current = await readFile(path.join(root, file), 'utf8');
  const body = (s) => s.split('pub fn achievements(game: &Game)')[1].split('#[cfg(test)]')[0];
  const predicates = (s) =>
    s
      .match(/let predicates = \[([\s\S]*?)\];/)[1]
      .trim()
      .split('\n')
      .map((v) => v.trim());
  const oldPredicates = predicates(body(old));
  const newPredicates = predicates(body(current));
  // Exact new order: each predicate stays aligned with its corresponding feat ID.
  assert.deepEqual(newPredicates, [
    oldPredicates[0],
    oldPredicates[1],
    'score >= 50.0,',
    oldPredicates[8],
    oldPredicates[2],
    oldPredicates[3],
    oldPredicates[10],
    oldPredicates[4],
    oldPredicates[5],
    oldPredicates[9],
    oldPredicates[6],
    oldPredicates[7],
  ]);
  const withoutPredicates = (s) => s.replace(/let predicates = \[[\s\S]*?\];/, 'PREDICATES');
  assert.equal(withoutPredicates(body(current)), withoutPredicates(body(old)));
  const artworkFile = 'web-client/release/achievement-assets.json';
  const previousArtwork = JSON.parse(
    execFileSync('git', ['show', `${baseline}:${artworkFile}`], { cwd: root, encoding: 'utf8' }),
  );
  const currentArtwork = JSON.parse(await readFile(path.join(root, artworkFile), 'utf8'));
  // Only the wrapper-version lockfile hash changes; artwork inputs stay identical.
  const lockfile = 'web-client/package-lock.json';
  assert.equal(
    currentArtwork.integrity[lockfile],
    createHash('sha256')
      .update(await readFile(path.join(root, lockfile)))
      .digest('hex'),
  );
  previousArtwork.integrity[lockfile] = currentArtwork.integrity[lockfile];
  assert.deepEqual(currentArtwork, previousArtwork);
  for (const file of [
    'web-client/scripts/publish-achievement-assets.mjs',
    'web-client/scripts/achievement-asset-inputs.mjs',
  ])
    assert.equal(
      await readFile(path.join(root, file), 'utf8'),
      execFileSync('git', ['show', `${baseline}:${file}`], { cwd: root, encoding: 'utf8' }),
    );
});
test('generated engine schema, 46 definitions using 43 assets', async () => {
  // Test current local WASM, not a stale published archive; an explicit packed
  // engine can still be supplied during a future release verification.
  const engine = createRequire(import.meta.url)(process.env.CLASH_TEST_ENGINE ?? '../.engine/server.js');
  assert.ok(inspectEngine(engine).compatible);
  const m = engine.achievementDefinitions();
  assert.equal(m.revision, 8);
  assert.equal(m.achievements.length, 46);
  const baselineManifest = JSON.parse(
    await readFile(path.join(root, 'web-client/src/fixtures/achievements-revision-5.json'), 'utf8'),
  );
  const previous = new Map(baselineManifest.achievements.map((a) => [a.id, a]));
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
    ...baselineManifest.achievements.slice(0, 10).map((a) => a.id),
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
    ...baselineManifest.achievements.slice(21).map((a) => a.id),
  ];
  assert.deepEqual(
    m.achievements,
    expectedIds.map((id) =>
      id === 'score-50'
        ? {
            id,
            name: '50 Points',
            description: 'Finish a game with at least 50 victory points in the final state.',
            icon: { type: 'asset', key: 'achievement-score-50' },
          }
        : id === 'score-60'
          ? { ...previous.get(id), icon: { type: 'asset', key: 'achievement-score-60' } }
          : wonderNames.has(id)
            ? { ...previous.get(id), name: wonderNames.get(id) }
            : previous.get(id),
    ),
  );
  const factions = JSON.parse(
    await readFile(path.join(root, 'web-client/release/achievement-assets.json')),
  ).assets;
  const reused = {
    'five-objectives': 'three-objectives',
    'seven-objectives': 'three-objectives',
    'thirty-advances': 'twenty-advances',
  };
  assert.deepEqual(
    m.achievements.filter((a) => !Object.hasOwn(reused, a.id)).map((a) => a.id),
    [...ids, ...factions.map((a) => 'win-' + a.key.slice('achievement-faction-'.length))],
  );
  for (const a of m.achievements)
    assert.deepEqual(a.icon, {
      type: 'asset',
      key: Object.hasOwn(reused, a.id)
        ? `achievement-${reused[a.id]}`
        : ids.includes(a.id)
          ? `achievement-${a.id}`
          : `achievement-faction-${a.id.slice(4)}`,
    });
  const raw = await engine.init(2, [], {}, 'artwork-package-test', {});
  assert.deepEqual(engine.achievements(raw), []);
  assert.deepEqual(engine.timeIncrements(raw), [0, 0]);
});
test('all 28 unique outputs decode, dimensions and hashes match', async () => {
  const r = await validateOutputs();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    for (const a of r.assets) {
      const b = await readFile(path.join(outputDir, a.file));
      assert.deepEqual(
        await page.evaluate(async (b) => {
          const i = new Image();
          i.src = 'data:image/png;base64,' + b;
          await i.decode();
          return [i.naturalWidth, i.naturalHeight];
        }, b.toString('base64')),
        [256, 256],
      );
    }
  } finally {
    await browser.close();
  }
});

test('score pictograms render their exact numerals and preserve every existing SVG', async () => {
  for (const score of [30, 40, 50, 60]) {
    const vector = svg(`score-${score}`);
    assert.deepEqual(
      [...vector.matchAll(/<text\b[^>]*>([^<]*)<\/text>/g)].map((m) => m[1]),
      [String(score)],
    );
  }
  const previousSource = execFileSync('git', ['show', `${baseline}:web-client/scripts/nonfaction-art.mjs`], {
    cwd: root,
    encoding: 'utf8',
  });
  const previous = await import(
    `data:text/javascript;base64,${Buffer.from(previousSource).toString('base64')}`
  );
  for (const id of previous.ids) assert.equal(svg(id), previous.svg(id), `${id} must remain unchanged`);
  assert.equal(new Set([40, 50, 60].map((score) => svg(`score-${score}`))).size, 3);
});
