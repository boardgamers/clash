import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { persistBackup, run } from './publish-achievement-assets.mjs';
import { sha256 } from './achievement-asset-inputs.mjs';

async function temporary(t) {
  const directory = await mkdtemp(path.join(tmpdir(), 'achievement-backups-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  return directory;
}

test('real backups are private, unique across retries, and retain original snapshot bytes/hashes', async (t) => {
  const base = path.join(await temporary(t), 'releases', 'achievement-assets');
  const original = { meta: { gallery: ['original'], assets: {} }, version: { version: 1, assets: {} } };
  const first = await persistBackup(original, base);
  const bytes = {};
  assert.equal((await stat(first.directory)).mode & 0o777, 0o700);
  assert.match(path.basename(first.directory), /^run-/);
  assert.equal(first.receiptPath, path.join(first.directory, 'upload-receipt.json'));
  for (const name of ['meta', 'version']) {
    bytes[name] = await readFile(first[name].path);
    assert.equal((await stat(first[name].path)).mode & 0o777, 0o600);
    assert.deepEqual(JSON.parse(bytes[name]), original[name]);
    assert.equal(sha256(bytes[name]), first[name].sha256);
  }
  original.meta.assets.changed = { url: 'new' };
  const second = await persistBackup(original, base);
  assert.notEqual(first.directory, second.directory);
  for (const name of ['meta', 'version']) {
    assert.deepEqual(await readFile(first[name].path), bytes[name]);
    assert.equal(sha256(await readFile(first[name].path)), first[name].sha256);
  }
  assert.notEqual(first.meta.sha256, second.meta.sha256);
});

test('real backup persistence rejects an unusable destination', async (t) => {
  const file = path.join(await temporary(t), 'not-a-directory');
  await writeFile(file, 'preserve');
  await assert.rejects(persistBackup({ meta: {}, version: {} }, file));
  assert.equal(await readFile(file, 'utf8'), 'preserve');
});

function fixture() {
  const key = 'achievement-faction-rome';
  const png = Buffer.from('frozen fixture');
  const manifest = {
    assets: [
      {
        key,
        civilization: 'Rome',
        source: { type: 'lucide', sha256: sha256(png), provenance: { license: 'ISC' } },
      },
    ],
  };
  const receipt = {
    revision: 2,
    assets: [{ key, civilization: 'Rome', file: `${key}.png`, sha256: sha256(png), width: 256, height: 256 }],
  };
  return {
    manifest,
    receipt,
    receiptBytes: Buffer.from(JSON.stringify(receipt)),
    files: new Map([[key, png]]),
  };
}

for (const failure of ['none', 'backup', 'save']) {
  test(`publication awaits backup and initial receipt before PUT (${failure})`, async () => {
    const local = fixture();
    const events = [];
    const meta = { gallery: ['original'] };
    const version = { version: 1 };
    const backup = {
      directory: '/private/run-test',
      receiptPath: '/private/run-test/upload-receipt.json',
      meta: { path: '/private/run-test/meta.json', sha256: 'a'.repeat(64) },
      version: { path: '/private/run-test/version.json', sha256: 'b'.repeat(64) },
    };
    let saved;
    await assert.rejects(
      run(['--publish', `--reviewed-sha256=${sha256(local.receiptBytes)}`], {
        loadLocal: async () => local,
        readToken: async () => 'secret-test-token',
        fetch: async (url, options) => {
          if (options.method === 'PUT') {
            events.push('PUT');
            throw Error('Stop after observing first PUT; no network');
          }
          events.push(url.endsWith('/meta') ? 'GET meta' : 'GET version');
          return { ok: true, json: async () => (url.endsWith('/meta') ? meta : version) };
        },
        persistBackup: async (sources) => {
          assert.deepEqual(sources, { meta, version });
          events.push('backup start');
          await new Promise((resolve) => setImmediate(resolve));
          if (failure === 'backup') throw Error('disk failure');
          events.push('backup complete');
          return backup;
        },
        save: async (evidence) => {
          events.push('save start');
          await new Promise((resolve) => setImmediate(resolve));
          if (failure === 'save') throw Error('receipt failure');
          saved = structuredClone(evidence);
          events.push('save complete');
        },
      }),
      /Publication\/verification failed/,
    );
    const expected = ['GET meta', 'GET version', 'backup start'];
    if (failure !== 'backup') expected.push('backup complete', 'save start');
    if (failure === 'none') expected.push('save complete', 'PUT');
    assert.deepEqual(events, expected);
    if (saved) {
      assert.deepEqual(saved.backup, backup);
      assert.equal(saved.status, 'incomplete');
      assert.ok(!JSON.stringify(saved).includes('secret-test-token'));
      assert.ok(!JSON.stringify(saved).includes('Authorization'));
    }
  });
}
