import test from 'node:test';
import assert from 'node:assert/strict';
import {
  run,
  licenseBlockers,
  validateReview,
  validateHostedAsset,
  assertPreserved,
} from './publish-achievement-assets.mjs';
import { sha256 } from './achievement-asset-inputs.mjs';

const key = 'achievement-faction-rome';
const png = Buffer.from('frozen PNG fixture; real PNG validation is performed by validateOutputs');
const webp = Buffer.from('RIFF\x08\x00\x00\x00WEBPtest');
const hosted = {
  url: `https://s3.fr-par.scw.cloud/bgs-assets/achievements/clash/${key}/${sha256(webp).slice(0, 16)}/${key}.webp`,
  size: webp.length,
  contentType: 'image/webp',
};
function fixture(type = 'lucide') {
  const source = { type, sha256: sha256(png), provenance: { license: 'ISC' } };
  const manifest = { assets: [{ key, civilization: 'Rome', source }] };
  const receipt = {
    revision: 2,
    manifestSha256: sha256(JSON.stringify(manifest)),
    assets: [{ key, civilization: 'Rome', file: `${key}.png`, sha256: sha256(png), width: 256, height: 256 }],
  };
  return {
    manifest,
    receipt,
    receiptBytes: Buffer.from(JSON.stringify(receipt)),
    files: new Map([[key, png]]),
  };
}
const publishArgs = (local) => ['--publish', `--reviewed-sha256=${sha256(local.receiptBytes)}`];
function offlineDeps(local) {
  return {
    loadLocal: async () => local,
    readToken: () => {
      throw Error('credential read forbidden');
    },
    fetch: () => {
      throw Error('network forbidden');
    },
  };
}

test('default dry-run is offline, does not read credentials and reports unresolved PNG provenance', async () => {
  const local = fixture('png');
  local.manifest.assets[0].source.provenance.license = null;
  const result = await run([], offlineDeps(local));
  assert.equal(result.mode, 'offline-dry-run');
  assert.equal(result.blockers.length, 3);
  assert.equal(result.receiptSha256, sha256(local.receiptBytes));
});

test('publication refuses missing review, mismatched review, missing licenses before credentials/network', async () => {
  const local = fixture();
  await assert.rejects(run(['--publish'], offlineDeps(local)), /reviewed receipt/);
  await assert.rejects(
    run(['--publish', `--reviewed-sha256=${'0'.repeat(64)}`], offlineDeps(local)),
    /hash mismatch/,
  );
  local.manifest.assets[0].source.provenance.license = null;
  await assert.rejects(run(publishArgs(local), offlineDeps(local)), /Unresolved source licenses/);
});

test('PNG clearance requires source URL, exact license/source hash match, and evidence', () => {
  const { manifest } = fixture('png');
  const source = manifest.assets[0].source;
  source.provenance = {
    license: 'CC0-1.0',
    sourceUrl: 'https://example.org/art/rome',
    verification: {
      sourceUrl: 'https://example.org/art/rome',
      sourceSha256: source.sha256,
      license: 'CC0-1.0',
      evidence: 'Reviewer matched artwork and license on the source page.',
    },
  };
  assert.deepEqual(licenseBlockers(manifest), []);
  for (const field of ['sourceUrl', 'sourceSha256', 'license', 'evidence']) {
    const changed = structuredClone(manifest);
    changed.assets[0].source.provenance.verification[field] = '';
    assert.ok(licenseBlockers(changed).length > 0, field);
  }
});

test('review rejects changed receipt, unexpected key and invalid dimensions', () => {
  const local = fixture();
  const validate = () =>
    validateReview(local.manifest, local.receipt, local.receiptBytes, sha256(local.receiptBytes));
  validate();
  local.receipt.assets[0].width = 255;
  assert.throws(validate, /Receipt changed/);
  local.receiptBytes = Buffer.from(JSON.stringify(local.receipt));
  assert.throws(validate);
  local.receipt.assets[0].width = 256;
  local.receipt.assets[0].key = 'unrelated-key';
  local.receiptBytes = Buffer.from(JSON.stringify(local.receipt));
  assert.throws(validate, /Unexpected asset key/);
});

test('frozen output mismatch blocks all network', async () => {
  const local = fixture();
  local.files.set(key, Buffer.from('changed'));
  await assert.rejects(run(publishArgs(local), offlineDeps(local)), /Frozen output hash mismatch/);
});

test('URL validation fails closed for HTTP, signatures, credentials, wrong namespace/key, path tricks and sizes', () => {
  assert.equal(validateHostedAsset(hosted, key), sha256(webp).slice(0, 16));
  for (const url of [
    hosted.url.replace('https:', 'http:'),
    hosted.url + '?token=secret',
    hosted.url + '#fragment',
    hosted.url.replace('s3.fr-par.scw.cloud', 'user:pass@s3.fr-par.scw.cloud'),
    hosted.url.replace('/achievements/', '/gallery/'),
    hosted.url.replace('/clash/', '/other/'),
    hosted.url.replaceAll(key, 'achievement-faction-greece'),
    hosted.url.replace('/clash/', '/%63lash/'),
    hosted.url.replace('s3.fr-par.scw.cloud', '127.0.0.1'),
    hosted.url.replace('s3.fr-par.scw.cloud', 'localhost'),
    hosted.url.replace('/bgs-assets/', '/'),
    hosted.url.replace('s3.fr-par.scw.cloud', 'untrusted.example'),
  ])
    assert.throws(() => validateHostedAsset({ ...hosted, url }, key));
  assert.throws(() => validateHostedAsset(hosted, 'unexpected-key'));
  assert.throws(() => validateHostedAsset({ ...hosted, size: '16' }, key));
  assert.throws(() => validateHostedAsset({ ...hosted, contentType: 'image/png' }, key));
});

test('preservation checks compare full GETs including gallery and unrelated assets', () => {
  const before = {
    title: 'Clash',
    gallery: [{ asset: 'cover' }],
    assets: { cover: { url: 'original' } },
    updatedAt: 1,
  };
  const after = structuredClone(before);
  after.updatedAt = 2;
  after.assets[key] = hosted;
  assertPreserved(before, after, [key]);
  for (const change of [
    (v) => v.gallery.push({ asset: key }),
    (v) => (v.title = 'changed'),
    (v) => (v.assets.cover.url = 'changed'),
    (v) => (v.assets.unexpected = hosted),
  ]) {
    const altered = structuredClone(after);
    change(altered);
    assert.throws(() => assertPreserved(before, altered, [key]));
  }
});

function mockPublication(local, mutate = () => {}) {
  const calls = [],
    saved = [];
  const baseline = { title: 'Clash', gallery: [], assets: { unrelated: { url: 'keep' } } };
  let uploaded = false;
  const current = () => ({
    ...baseline,
    assets: { ...baseline.assets, ...(uploaded ? { [key]: hosted } : {}) },
  });
  const deps = {
    loadLocal: async () => local,
    readToken: async () => 'SECRET-DO-NOT-LOG',
    save: async (value) => saved.push(structuredClone(value)),
    fetch: async (url, options) => {
      calls.push({ url, options });
      let value;
      if (options.method === 'PUT') {
        uploaded = true;
        value = current();
      } else if (url === hosted.url) {
        const response = new Response(webp, { headers: { 'Content-Type': 'image/webp' } });
        return mutate(url, response) ?? response;
      } else value = current();
      value = mutate(url, value) ?? value;
      return new Response(JSON.stringify(value), { headers: { 'Content-Type': 'application/json' } });
    },
  };
  return { deps, calls, saved };
}

test('mock publication uploads only reviewed PNGs, downloads anonymously, verifies full GETs, saves hashes without token', async () => {
  const local = fixture();
  const { deps, calls, saved } = mockPublication(local);
  const result = await run(publishArgs(local), deps);
  assert.equal(result.status, 'verified');
  assert.equal(result.assets[0].sha256, sha256(webp));
  assert.equal(result.reviewedReceiptSha256, sha256(local.receiptBytes));
  const writes = calls.filter((c) => c.options.method === 'PUT');
  assert.equal(writes.length, 1);
  assert.equal(
    writes[0].url,
    `https://admin.boardgamers.space/api/admin/gameinfo/clash/assets/${key}?name=${key}.png`,
  );
  assert.equal(writes[0].options.headers['Content-Type'], 'image/png');
  assert.deepEqual(writes[0].options.body, png);
  for (const call of calls.filter((c) => !c.url.includes('/api/admin/'))) {
    assert.equal(call.options.headers, undefined);
    assert.equal(call.options.credentials, 'omit');
    assert.equal(call.options.redirect, 'error');
  }
  assert.equal(calls.filter((c) => c.url.endsWith('/meta')).length, 2);
  assert.equal(calls.filter((c) => c.url.endsWith('/clash/1')).length, 2);
  assert.ok(calls.some((c) => c.url.endsWith('/api/boardgame/clash/info/latest')));
  assert.ok(!JSON.stringify(saved).includes('SECRET'));
  assert.equal(saved.at(-1).status, 'verified');
});

test('bad download hashes, content type, length, response key and public info mismatch fail verification', async () => {
  for (const mutate of [
    (url, value) =>
      url === hosted.url
        ? new Response(Buffer.from('RIFF\x08\x00\x00\x00WEBPfail'), {
            headers: { 'Content-Type': 'image/webp' },
          })
        : value,
    (url, value) =>
      url === hosted.url ? new Response(webp, { headers: { 'Content-Type': 'image/png' } }) : value,
    (url, value) =>
      url === hosted.url
        ? new Response(webp.subarray(0, 12), { headers: { 'Content-Type': 'image/webp' } })
        : value,
    (url, value) => (url.includes('?name=') ? { assets: { unexpected: hosted } } : value),
    (url, value) => (url.endsWith('/info/latest') ? { assets: {} } : value),
  ]) {
    const local = fixture();
    const { deps, saved } = mockPublication(local, mutate);
    await assert.rejects(run(publishArgs(local), deps), /Publication\/verification failed/);
    assert.notEqual(saved.at(-1)?.status, 'verified');
  }
});

test('remote errors are not leaked even when fetch includes Authorization in its exception', async () => {
  const local = fixture();
  const { deps } = mockPublication(local);
  deps.fetch = async (_url, options) => {
    throw Error(JSON.stringify(options));
  };
  await assert.rejects(
    run(publishArgs(local), deps),
    (error) => !error.message.includes('SECRET') && !error.stack.includes('Bearer'),
  );
});

test('unknown flags and contradictory modes fail closed', async () => {
  for (const args of [['--force'], ['--publish', '--dry-run'], ['--reviewed-sha256=bad']]) {
    await assert.rejects(
      run(args, {
        loadLocal: () => {
          throw Error('should not load');
        },
      }),
      /argument|modes/,
    );
  }
});
