import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import type { Controller as ControllerType } from './controller';
import type { Session } from './types';

const engine = createRequire(import.meta.url)('../.engine/server.js');
// Bundle the actual controller so Node can run its browser TypeScript imports.
// Only the WASM loader changes: the tests use the same engine's Node bridge.
const bundled = await build({
  entryPoints: [fileURLToPath(new URL('./controller.ts', import.meta.url))],
  bundle: true,
  write: false,
  platform: 'node',
  format: 'esm',
  logLevel: 'silent',
  plugins: [
    {
      name: 'node-engine',
      setup(plugin) {
        plugin.onResolve({ filter: /^\.\/bridge$/ }, () => ({ path: 'bridge', namespace: 'test-engine' }));
        plugin.onLoad({ filter: /.*/, namespace: 'test-engine' }, () => ({
          contents: `import {createRequire} from 'node:module'; const engine=createRequire(${JSON.stringify(import.meta.url)})('../.engine/server.js'); export async function loadBridge(){return engine;}`,
        }));
      },
    },
  ],
});
const { Controller } = (await import(
  `data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString('base64')}`
)) as { Controller: typeof ControllerType };

test('Expansion keeps its new settler and destination selected across repeated platform metadata and snapshots', async () => {
  let submitted = '';
  const controller = new Controller(
    {
      move: (move: string) => {
        submitted = move;
        return true;
      },
      replaceLog: () => {},
      fetchState: () => {},
    } as unknown as ControllerType['commands'],
    new URL('http://localhost/'),
  );
  let session: Session;
  const off = controller.session.subscribe((s) => {
    session = s;
  });
  try {
    const raw = JSON.parse(
      await engine.init(2, [], { undo: 'SamePlayer', civilization: 'Random' }, 'clash-preview-20260927', {}),
    );
    const seat = engine.currentPlayer(JSON.stringify(raw));
    raw.players[seat].civilization = 'China';
    raw.players[seat].advances.push('Husbandry');
    raw.players[seat].special_advances = ['Expansion'];
    let state = JSON.stringify(raw);
    const load = () => controller.load(engine.stripSecret(state, seat));
    controller.setPlayer(seat);
    await load();
    const city = session!.city!;
    const oldIds = session!.view!.units!.map((u) => u.id);
    controller.openCities(city, 'recruit');
    controller.setRecruits({ settlers: 1 });
    assert.ok(session!.recruitPreview?.action);
    controller.submit(session!.recruitPreview!.action);
    state = engine.tryMove(state, submitted, seat);
    await load();
    const newSettler = session!.view!.units!.find((u) => !oldIds.includes(u.id))!;
    assert.equal(session!.mode, 'settlers');
    assert.deepEqual(session!.selectedUnits, [newSettler.id]);
    assert.ok(session!.moveDestinations.length);
    const target = session!.moveDestinations[0].position;
    controller.selectTile(target);
    const selection = session!.moveDestination;
    controller.setPlayer(seat);
    await load();
    assert.equal(session!.mode, 'settlers');
    assert.deepEqual(session!.selectedUnits, [newSettler.id]);
    assert.equal(session!.moveTarget, target);
    assert.equal(session!.moveDestination, selection);
    // The city itself also resumes movement after intentionally closing the controls.
    controller.patch({ mode: 'overview', tilePanel: false });
    controller.selectTile(city, { kind: 'city', player: seat });
    assert.equal(session!.mode, 'settlers');
    assert.ok(session!.moveDestinations.length);
    // Duplicate snapshots must not acknowledge an action the server has not applied.
    controller.submit(session!.moveDestinations[0].action);
    await load();
    assert.equal(session!.pending, true);
  } finally {
    off();
    controller.destroy();
  }
});
