import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
import { inspectEngine } from '@boardgamers/protocol/engine';
import { ids } from './nonfaction-art.mjs';
import { validateOutputs, outputDir } from './nonfaction-assets.mjs';
import { root } from './achievement-asset-inputs.mjs';
// Last source release before the three additive revision-5 milestones.
const baseline = '59fd2671380391889d1dff0e0f15db16fdc215ff';
test('existing evaluation and faction assets are unchanged apart from the three new milestones', async () => {
  const file = 'server/src/achievements.rs';
  const old = execFileSync('git', ['show', `${baseline}:${file}`], { cwd: root, encoding: 'utf8' });
  const current = await readFile(path.join(root, file), 'utf8');
  const body = (s) => s.split('pub fn achievements(game: &Game)')[1].split('#[cfg(test)]')[0];
  // Only these three inclusive predicates may change evaluation.
  let evaluator = body(current);
  for (const predicate of [
    '                score >= 60.0,\n',
    '                p.completed_objectives.len() >= 7,\n',
    '                p.advances.len() >= 30,\n',
  ]) {
    assert.equal(evaluator.split(predicate).length, 2);
    evaluator = evaluator.replace(predicate, '');
  }
  assert.equal(evaluator, body(old));
  const artworkFile = 'web-client/release/achievement-assets.json';
  const previousArtwork = JSON.parse(
    execFileSync('git', ['show', `${baseline}:${artworkFile}`], { cwd: root, encoding: 'utf8' }),
  );
  const currentArtwork = JSON.parse(await readFile(path.join(root, artworkFile), 'utf8'));
  // Only the wrapper-version lockfile integrity pin changes; artwork is identical.
  previousArtwork.integrity['web-client/package-lock.json'] =
    currentArtwork.integrity['web-client/package-lock.json'];
  assert.deepEqual(currentArtwork, previousArtwork);
  for (const file of [
    'web-client/release/achievement-assets-nonfaction.json',
    'web-client/scripts/publish-achievement-assets.mjs',
    'web-client/scripts/achievement-asset-inputs.mjs',
  ])
    assert.equal(
      await readFile(path.join(root, file), 'utf8'),
      execFileSync('git', ['show', `${baseline}:${file}`], { cwd: root, encoding: 'utf8' }),
    );
});
test('generated engine schema, 45 definitions sharing 41 existing assets', async () => {
  // Test current local WASM, not a stale published archive; an explicit packed
  // engine can still be supplied during a future release verification.
  const engine = createRequire(import.meta.url)(process.env.CLASH_TEST_ENGINE ?? '../.engine/server.js');
  assert.ok(inspectEngine(engine).compatible);
  const m = engine.achievementDefinitions();
  assert.equal(m.revision, 5);
  assert.equal(m.achievements.length, 45);
  const factions = JSON.parse(
    await readFile(path.join(root, 'web-client/release/achievement-assets.json')),
  ).assets;
  const reused = {
    'five-objectives': 'three-objectives',
    'score-60': 'score-40',
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
test('all 26 unique outputs decode, dimensions and hashes match', async () => {
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
