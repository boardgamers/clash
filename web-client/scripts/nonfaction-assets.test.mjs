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
const baseline = '0099fa11cd697080bc8dccda629d8d4f5c92aae5';
test('evaluation body and existing faction manifest remain byte identical', async () => {
  const file = 'server/src/achievements.rs';
  const old = execFileSync('git', ['show', `${baseline}:${file}`], { cwd: root, encoding: 'utf8' });
  const current = await readFile(path.join(root, file), 'utf8');
  const body = (s) => s.split('pub fn achievements(game: &Game)')[1].split('#[cfg(test)]')[0];
  assert.equal(body(current), body(old));
  for (const file of [
    'web-client/release/achievement-assets.json',
    'web-client/scripts/publish-achievement-assets.mjs',
    'web-client/scripts/achievement-asset-inputs.mjs',
  ])
    assert.equal(
      await readFile(path.join(root, file), 'utf8'),
      execFileSync('git', ['show', `${baseline}:${file}`], { cwd: root, encoding: 'utf8' }),
    );
});
test('exact packed engine schema, complete stable catalogue and 26 asset refs', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'clash-packed-'));
  execFileSync('tar', [
    'xzf',
    path.join(root, 'web-client/.engine/releases/boardgamers-clash-server-0.4.152.tgz'),
    '-C',
    dir,
  ]);
  const engine = createRequire(import.meta.url)(path.join(dir, 'package/server.js'));
  assert.ok(inspectEngine(engine).compatible);
  const m = engine.achievementDefinitions();
  assert.equal(m.revision, 3);
  assert.equal(m.achievements.length, 41);
  const factions = JSON.parse(
    await readFile(path.join(root, 'web-client/release/achievement-assets.json')),
  ).assets;
  assert.deepEqual(
    m.achievements.map((a) => a.id),
    [...ids, ...factions.map((a) => 'win-' + a.key.slice('achievement-faction-'.length))],
  );
  for (const a of m.achievements)
    assert.deepEqual(a.icon, {
      type: 'asset',
      key: ids.includes(a.id) ? `achievement-${a.id}` : `achievement-faction-${a.id.slice(4)}`,
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
