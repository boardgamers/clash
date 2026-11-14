// Offline by default. This tool only writes named assets, never game metadata,
// gallery, engine releases, activation, or viewer configuration.
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { validateInputs, validateOutputs, outputDir, sha256 } from './achievement-asset-inputs.mjs';

const admin = 'https://admin.boardgamers.space/api/admin';
const publicInfo = 'https://boardgamers.space/api/boardgame/clash/info/latest';
const nonempty = (value) => typeof value === 'string' && value.trim().length > 0;
const hashPattern = /^[a-f0-9]{64}$/;
const keyPattern = /^achievement-faction-[a-z]+$/;

function httpsUrl(value) {
  try {
    const url = new URL(value);
    return url.protocol === 'https:' && !url.username && !url.password && !url.search && !url.hash;
  } catch {
    return false;
  }
}

// License clearance is a human review, not inferred from a filename or an icon
// site's general license. PNG provenance must contain:
// license: nonempty license identifier/text; sourceUrl: exact HTTPS artwork URL;
// verification: { sourceUrl, sourceSha256, license, evidence }.
// The first three verification fields must match this source exactly; evidence
// is a nonempty reviewer-authored citation/explanation of the artwork match and
// license check. Changing provenance requires regenerating/reviewing the receipt.
export function licenseBlockers(manifest) {
  return manifest.assets.flatMap(({ key, source }) => {
    const p = source.provenance ?? {};
    const errors = [];
    if (!nonempty(p.license)) errors.push('missing license');
    if (source.type === 'png') {
      if (!httpsUrl(p.sourceUrl)) errors.push('missing unsigned HTTPS sourceUrl');
      const v = p.verification;
      if (
        !v ||
        v.sourceUrl !== p.sourceUrl ||
        v.sourceSha256 !== source.sha256 ||
        v.license !== p.license ||
        !nonempty(v.evidence)
      ) {
        errors.push('missing matched verification evidence');
      }
    } else if (source.type !== 'lucide') errors.push('unsupported source type');
    return errors.map((reason) => ({ key, reason }));
  });
}

export function validateReview(manifest, receipt, receiptBytes, reviewedSha256) {
  assert.ok(hashPattern.test(reviewedSha256 ?? ''), 'Explicit reviewed receipt SHA-256 required');
  assert.equal(sha256(receiptBytes), reviewedSha256, 'Reviewed receipt hash mismatch');
  assert.deepEqual(JSON.parse(receiptBytes.toString()), receipt, 'Receipt changed during validation');
  assert.equal(receipt.revision, 2);
  assert.deepEqual(
    receipt.assets.map(({ key, civilization }) => ({ key, civilization })),
    manifest.assets.map(({ key, civilization }) => ({ key, civilization })),
    'Unexpected asset key',
  );
  assert.equal(new Set(receipt.assets.map((a) => a.key)).size, receipt.assets.length);
  assert.ok(receipt.assets.length > 0);
  for (const asset of receipt.assets) {
    assert.ok(keyPattern.test(asset.key), 'Unexpected asset key');
    assert.equal(asset.file, `${asset.key}.png`);
    assert.ok(hashPattern.test(asset.sha256), 'Invalid local hash');
    assert.equal(asset.width, 256);
    assert.equal(asset.height, 256);
  }
  assert.equal(licenseBlockers(manifest).length, 0, 'Unresolved source licenses; publication blocked');
}

export function validateHostedAsset(asset, key) {
  assert.ok(keyPattern.test(key), 'Unexpected asset key');
  assert.ok(asset && httpsUrl(asset.url), 'Hosted URL must be unsigned HTTPS');
  const url = new URL(asset.url);
  // Production stores public objects below the bucket path, not the host root.
  // A storage endpoint change requires review; never follow arbitrary returned URLs.
  assert.equal(url.origin, 'https://s3.fr-par.scw.cloud', 'Unexpected storage origin');
  assert.equal(asset.contentType, 'image/webp', 'Unexpected asset metadata content type');
  const match = url.pathname.match(
    new RegExp(`^/bgs-assets/achievements/clash/${key}/([a-f0-9]{16})/${key}\\.webp$`),
  );
  assert.ok(match, 'Unexpected hosted asset namespace/key');
  assert.ok(Number.isSafeInteger(asset.size) && asset.size > 0, 'Invalid hosted size');
  return match[1];
}

export function assertPreserved(before, after, keys) {
  const unrelated = (value) => {
    const copy = structuredClone(value);
    delete copy.updatedAt; // Server-maintained timestamp may change on named-asset writes.
    for (const key of keys) if (copy.assets) delete copy.assets[key];
    return copy;
  };
  assert.deepEqual(unrelated(after), unrelated(before), 'Unrelated metadata/gallery/assets changed');
}

async function loadLocal() {
  const manifest = await validateInputs();
  const receipt = await validateOutputs();
  const receiptBytes = await readFile(path.join(outputDir, 'receipt.json'));
  // Freeze every upload in memory before credentials or network are touched.
  const files = new Map();
  for (const asset of receipt.assets) {
    assert.equal(asset.file, `${asset.key}.png`);
    const bytes = await readFile(path.join(outputDir, asset.file));
    assert.equal(sha256(bytes), asset.sha256, 'Output changed during validation');
    files.set(asset.key, bytes);
  }
  return { manifest, receipt, receiptBytes, files };
}

export async function run(args = [], dependencies = {}) {
  assert.ok(
    args.every((a) => a === '--publish' || a === '--dry-run' || /^--reviewed-sha256=[a-f0-9]{64}$/.test(a)),
    'Unknown or malformed argument',
  );
  assert.ok(!(args.includes('--publish') && args.includes('--dry-run')), 'Conflicting modes');
  assert.ok(args.filter((a) => a.startsWith('--reviewed-sha256=')).length <= 1, 'Duplicate review hash');
  const local = await (dependencies.loadLocal ?? loadLocal)();
  const { manifest, receipt, receiptBytes, files } = local;
  const blockers = licenseBlockers(manifest);
  const reviewedSha256 = args.find((a) => a.startsWith('--reviewed-sha256='))?.split('=')[1];
  const report = {
    mode: 'offline-dry-run',
    receiptSha256: sha256(receiptBytes),
    assets: receipt.assets.length,
    blockers,
  };
  if (!args.includes('--publish')) return report;
  validateReview(manifest, receipt, receiptBytes, reviewedSha256);
  for (const asset of receipt.assets) {
    assert.ok(Buffer.isBuffer(files.get(asset.key)), 'Missing frozen PNG');
    assert.equal(sha256(files.get(asset.key)), asset.sha256, 'Frozen output hash mismatch');
  }

  // Only after ALL local review/license checks pass do we read credentials.
  const token = (
    await (
      dependencies.readToken ??
      (async () => process.env.BGS_ADMIN_TOKEN ?? (await readFile(path.join(homedir(), '.bgs'), 'utf8')))
    )()
  ).trim();
  assert.ok(token.length > 0, 'Missing admin token');
  const fetcher = dependencies.fetch ?? globalThis.fetch;
  const save =
    dependencies.save ??
    ((data) =>
      writeFile(path.join(outputDir, 'upload-receipt.json'), JSON.stringify(data, null, 2) + '\n', {
        mode: 0o600,
      }));
  // Never echo remote bodies, errors, headers, or tokens (including fetch errors).
  try {
    async function request(url, options = {}) {
      const response = await fetcher(url, {
        redirect: 'error',
        signal: AbortSignal.timeout(60000),
        ...options,
      });
      if (!response.ok) throw new Error('Request failed');
      return response;
    }
    async function api(endpoint, body) {
      return (
        await request(`${admin}${endpoint}`, {
          method: body ? 'PUT' : 'GET',
          headers: { Authorization: `Bearer ${token}`, ...(body ? { 'Content-Type': 'image/png' } : {}) },
          ...(body ? { body } : {}),
        })
      ).json();
    }
    const before = await api('/gameinfo/clash/meta');
    const gameBefore = await api('/gameinfo/clash/1');
    const uploads = [];
    const evidence = {
      formatRevision: 1,
      achievementRevision: receipt.revision,
      reviewedReceiptSha256: reviewedSha256,
      assets: uploads,
      status: 'incomplete',
    };
    await save(evidence);
    for (const localAsset of receipt.assets) {
      const { key } = localAsset;
      const metadata = await api(
        `/gameinfo/clash/assets/${key}?name=${encodeURIComponent(key + '.png')}`,
        files.get(key),
      );
      const asset = metadata.assets?.[key];
      const prefix = validateHostedAsset(asset, key);
      // No Authorization header, cookies, or redirects for public verification.
      const hosted = await request(asset.url, { credentials: 'omit' });
      assert.equal(hosted.headers.get('content-type')?.split(';')[0].trim().toLowerCase(), 'image/webp');
      const bytes = Buffer.from(await hosted.arrayBuffer());
      assert.equal(bytes.length, asset.size);
      assert.equal(bytes.subarray(0, 4).toString(), 'RIFF');
      assert.equal(bytes.subarray(8, 12).toString(), 'WEBP');
      const hash = sha256(bytes);
      assert.equal(hash.slice(0, 16), prefix);
      uploads.push({ key, url: asset.url, size: asset.size, sha256: hash, localSha256: localAsset.sha256 });
      await save(evidence);
    }
    const after = await api('/gameinfo/clash/meta');
    const gameAfter = await api('/gameinfo/clash/1');
    const keys = receipt.assets.map((a) => a.key);
    assertPreserved(before, after, keys);
    assertPreserved(gameBefore, gameAfter, keys);
    const published = await (await request(publicInfo, { credentials: 'omit' })).json();
    for (const asset of uploads) {
      for (const info of [after, gameAfter, published]) {
        assert.equal(info.assets?.[asset.key]?.url, asset.url);
        assert.equal(info.assets?.[asset.key]?.size, asset.size);
      }
    }
    evidence.status = 'verified';
    await save(evidence);
    return evidence;
  } catch {
    throw new Error(
      'Publication/verification failed; uploads may be partial. Inspect the local upload receipt; no activation was performed.',
    );
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    console.log(JSON.stringify(await run(process.argv.slice(2)), null, 2));
  } catch {
    console.error(
      'Achievement asset validation/publication failed. Run offline dry-run to check inputs and license blockers. No activation was performed.',
    );
    process.exitCode = 1;
  }
}
