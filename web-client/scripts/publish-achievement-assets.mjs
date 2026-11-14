// Offline by default. This tool only writes named assets, never game metadata,
// gallery, engine releases, activation, or viewer configuration.
import assert from 'node:assert/strict';
import { readFile, mkdir, mkdtemp, chmod, open, rename, rmdir } from 'node:fs/promises';
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

// Narrow, user-authorized reuse of the exact four symbols already served by Clash.
// These pins establish project identity/history, NOT per-item license clearance.
const existingProjectPngs = {
  'achievement-faction-rome': [
    'client/assets/colosseum-rome-svgrepo-com.png',
    '660f60ac288e2ed97ae208fce824ec4b39681008b68429b82b76f9dfeefb6d67',
    '603be9b8f0a361bebcef69d019c6451d75255eb8',
  ],
  'achievement-faction-greece': [
    'client/assets/temple-building-with-columns-svgrepo-com.png',
    '939ab2c36bb3f29aa066348f5ea0f8418e76442f1fcae1537a27ec1706a57e0f',
    '7fc48ab4187c9d8bb874b11ec2581ce1b4a1fc97',
  ],
  'achievement-faction-china': [
    'client/assets/great-wall-of-china-chinese-svgrepo-com.png',
    'edc3bfe45c937482fedb708bcd794ed589241f96b095a55daa25be7bb4656e3c',
    '603be9b8f0a361bebcef69d019c6451d75255eb8',
  ],
  'achievement-faction-vikings': [
    'client/assets/viking-ship-svgrepo-com.png',
    'a195e2fcafbb2338a6b79c05f55f6c3e5aa64fd68352c30f82e13b8d9094274b',
    '77bfc205eec2b664b24846753e0680ebf6ec9740',
  ],
};
const existingViewer =
  'https://s3.fr-par.scw.cloud/bgs-assets/games/clash/1/viewer/assets-890e8364e4f3fe49236b/viewer.js';

// Other PNGs still need matched source/license evidence, not a filename or a
// site's general license. Their provenance must contain:
// license: nonempty license identifier/text; sourceUrl: exact HTTPS artwork URL;
// verification: { sourceUrl, sourceSha256, license, evidence }.
// The first three verification fields must match this source exactly; evidence
// is a nonempty reviewer-authored citation/explanation of the artwork match and
// license check. Changing provenance requires regenerating/reviewing the receipt.
export function licenseBlockers(manifest) {
  return manifest.assets.flatMap(({ key, source }) => {
    const p = source.provenance ?? {};
    const errors = [];
    if (p.type === 'verified-existing-project-asset') {
      const pin = existingProjectPngs[key];
      const valid =
        source.type === 'png' &&
        pin &&
        source.path === pin[0] &&
        source.sha256 === pin[1] &&
        p.sourcePath === pin[0] &&
        p.sourceSha256 === pin[1] &&
        p.historicalCommit === pin[2] &&
        p.license === null &&
        p.licenseStatus === 'per-item-license-unverified' &&
        p.existingUse?.viewerUrl === existingViewer &&
        p.existingUse?.embeddedPngSha256 === pin[1] &&
        nonempty(p.existingUse?.evidence) &&
        p.authorization === 'explicit-user-request-to-reuse-established-project-artwork';
      return valid ? [] : [{ key, reason: 'invalid verified existing project asset evidence' }];
    }
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

async function syncDirectory(directory) {
  const handle = await open(directory, 'r');
  try {
    await handle.sync();
  } finally {
    await handle.close();
  }
}

async function writeExclusive(file, bytes) {
  const handle = await open(file, 'wx', 0o600);
  try {
    await handle.chmod(0o600);
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
}

// Only the two source JSON responses belong here: never request options or credentials.
// Each attempt gets a new directory; even failed attempts retain their originals.
export async function persistBackup(
  { meta, version },
  baseDirectory = path.join(path.dirname(outputDir), 'releases', 'achievement-assets'),
) {
  await mkdir(baseDirectory, { recursive: true, mode: 0o700 });
  const directory = await mkdtemp(path.join(baseDirectory, 'run-'));
  await chmod(directory, 0o700);
  const result = { directory, receiptPath: path.join(directory, 'upload-receipt.json') };
  for (const [name, value] of Object.entries({ meta, version })) {
    const bytes = Buffer.from(JSON.stringify(value, null, 2) + '\n');
    const file = path.join(directory, `${name}.json`);
    await writeExclusive(file, bytes);
    result[name] = { path: file, sha256: sha256(bytes) };
  }
  await syncDirectory(directory);
  await syncDirectory(baseDirectory);
  await syncDirectory(path.dirname(baseDirectory));
  return result;
}

async function saveBackupReceipt(backup, data) {
  // Exclusive temporary file + rename keeps the previous receipt intact on failure.
  const temporary = await mkdtemp(path.join(backup.directory, '.receipt-'));
  await chmod(temporary, 0o700);
  const file = path.join(temporary, 'receipt.json');
  await writeExclusive(file, JSON.stringify(data, null, 2) + '\n');
  await rename(file, backup.receiptPath);
  await rmdir(temporary);
  await syncDirectory(backup.directory);
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
    const backup = await (dependencies.persistBackup ?? persistBackup)({ meta: before, version: gameBefore });
    const save = dependencies.save ?? ((data) => saveBackupReceipt(backup, data));
    const uploads = [];
    const evidence = {
      formatRevision: 1,
      achievementRevision: receipt.revision,
      reviewedReceiptSha256: reviewedSha256,
      backup,
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
