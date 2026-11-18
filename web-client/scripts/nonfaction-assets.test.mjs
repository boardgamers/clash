import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile, mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { chromium } from 'playwright';
import { inspectEngine } from '@boardgamers/protocol/engine';
import { ids } from './nonfaction-art.mjs';
import { validateOutputs, outputDir } from './nonfaction-assets.mjs';
import { root } from './achievement-asset-inputs.mjs';
// Last source release before the additive five-objective tier.
const baseline = 'e69d7148fb38775898389a1a5eef458993d540b1';
test('existing evaluation and faction assets are unchanged apart from the new objective tier', async () => {
  const file = 'server/src/achievements.rs';
  const old = execFileSync('git', ['show', `${baseline}:${file}`], { cwd: root, encoding: 'utf8' });
  const current = await readFile(path.join(root, file), 'utf8');
  const body = (s) => s.split('pub fn achievements(game: &Game)')[1].split('#[cfg(test)]')[0];
  // The sole evaluator change is the inclusive five-record objective predicate.
  const addedPredicate = '                p.completed_objectives.len() >= 5,\n';
  assert.equal(body(current).split(addedPredicate).length, 2);
  assert.equal(body(current).replace(addedPredicate, ''), body(old));
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
    'web-client/scripts/publish-achievement-assets.mjs',
    'web-client/scripts/achievement-asset-inputs.mjs',
  ])
    assert.equal(
      await readFile(path.join(root, file), 'utf8'),
      execFileSync('git', ['show', `${baseline}:${file}`], { cwd: root, encoding: 'utf8' }),
    );
});
test('exact packed engine schema, 42 definitions sharing 41 existing assets', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'clash-packed-'));
  const metadata = JSON.parse(await readFile(path.join(root, 'web-client/release/engine-package.json')));
  execFileSync('tar', [
    'xzf',
    path.join(root, `web-client/.engine/releases/boardgamers-clash-server-${metadata.version}.tgz`),
    '-C',
    dir,
  ]);
  const engine = createRequire(import.meta.url)(path.join(dir, 'package/server.js'));
  assert.ok(inspectEngine(engine).compatible);
  const m = engine.achievementDefinitions();
  assert.equal(m.revision, 4);
  assert.equal(m.achievements.length, 42);
  const factions = JSON.parse(
    await readFile(path.join(root, 'web-client/release/achievement-assets.json')),
  ).assets;
  assert.deepEqual(
    m.achievements.filter((a) => a.id !== 'five-objectives').map((a) => a.id),
    [...ids, ...factions.map((a) => 'win-' + a.key.slice('achievement-faction-'.length))],
  );
  for (const a of m.achievements)
    assert.deepEqual(a.icon, {
      type: 'asset',
      key:
        a.id === 'five-objectives'
          ? 'achievement-three-objectives'
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
