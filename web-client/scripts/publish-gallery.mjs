// Gallery-only release: no engine, viewer, rules or general metadata writes.
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { homedir } from 'node:os';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';

const root = new URL('../', import.meta.url);
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
export function replaceGallery(gallery, images) {
  const replacements = new Map(images.filter(i => i.asset).map(i => [i.previousAsset, i.asset]));
  const result = gallery.map(item => ({...item,
    asset: replacements.get(item.asset) ?? item.asset,
    ...(item.darkAsset ? {darkAsset: replacements.get(item.darkAsset) ?? item.darkAsset} : {}),
  }));
  for (const image of images.filter(i => i.asset)) {
    assert.equal(result.filter(i => i.asset === image.asset || i.darkAsset === image.asset).length, 1,
      `Expected exactly one gallery reference to ${image.previousAsset}`);
  }
  return result;
}
export async function validateCaptures() {
  const manifest = JSON.parse(await readFile(new URL('release/screenshots/manifest.json', root)));
  assert.equal(manifest.images.length, 8);
  for (const image of manifest.images) {
    const bytes = await readFile(new URL(`release/screenshots/${image.file}`, root));
    assert.equal(bytes.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
    assert.equal(sha256(bytes), image.sha256, image.file);
    assert.equal(bytes.readUInt32BE(16), image.width);
    assert.equal(bytes.readUInt32BE(20), image.height);
  }
  const checks = JSON.parse(await readFile(new URL('release/screenshots/checks.json', root)));
  assert.deepEqual(checks.errors, []);
  assert.equal(checks.status, 'ended');
  assert.equal(checks.checks.length, 4);
  for (const check of checks.checks) {
    if (check.mobile) { assert.equal(check.mobile.moves, 0); assert.equal(check.mobile.width, check.mobile.scrollWidth); }
    if (check.diagnostics) assert.equal(check.diagnostics.compatible, true);
  }
  return manifest.images;
}
async function main() {
  const images = await validateCaptures();
  if (process.argv.includes('--check')) { console.log('Eight frozen captures validated.'); return; }
  const publish = process.argv.includes('--publish');
  const evidence = process.argv.find(a => a.startsWith('--evidence='))?.slice(11) ?? '/tmp/gallery-real-games-release';
  await mkdir(evidence, {recursive:true});
  const save = (name, data) => writeFile(`${evidence}/${name}.json`, JSON.stringify(data, null, 2) + '\n');
  const token = process.env.BGS_ADMIN_TOKEN ?? (await readFile(`${homedir()}/.bgs`, 'utf8')).trim();
  async function api(path, method = 'GET', body, type = 'application/json') {
    const response = await fetch(`https://admin.boardgamers.space/api/admin${path}`, {
      method, redirect:'error', signal:AbortSignal.timeout(60000),
      headers:{Authorization:`Bearer ${token}`, 'Content-Type':type},
      body:body === undefined ? undefined : type === 'application/json' ? JSON.stringify(body) : body,
    });
    assert(response.ok, `${method} ${path}: ${response.status}`);
    return response.json();
  }
  async function verifyRules() {
    const rules = JSON.parse(await readFile(new URL('release/rules.json', root)));
    const hashes = {};
    for (const [locale, content] of Object.entries(rules)) {
      assert.equal((await api(`/page/clash%3Arules/${locale}`)).content, content, `Rules drift: ${locale}`);
      hashes[locale] = sha256(content);
    }
    return hashes;
  }
  const before = await api('/gameinfo/clash/1');
  const metaBefore = await api('/gameinfo/clash/meta');
  const gallery = replaceGallery(before.gallery, images);
  await save('publication-before', before);
  await save('publication-meta-before', metaBefore);
  await save('rules-verified', await verifyRules());
  await save('planned-gallery', gallery);
  if (!publish) { console.log('Dry run: rules match source; gallery plan saved. Use --publish after main CI passes.'); return; }
  const downloads = [];
  for (const image of images.filter(i => i.asset)) {
    // Content-derived keys keep the currently selected gallery untouched until
    // every new image has been uploaded and independently downloaded/verified.
    const uploaded = await api(`/gameinfo/clash/assets/${image.asset}`, 'PUT',
      await readFile(new URL(`release/screenshots/${image.file}`, root)), 'image/png');
    const asset = uploaded.assets[image.asset];
    const response = await fetch(asset.url, {signal:AbortSignal.timeout(60000)});
    assert(response.ok, `Asset download: ${response.status}`);
    const bytes = Buffer.from(await response.arrayBuffer());
    // BGS intentionally converts PNG to WebP. Compare the exact deterministic
    // conversion, not a false claim of PNG byte identity.
    assert.equal(sha256(bytes), image.webpSha256, `Converted asset mismatch: ${image.file}`);
    await writeFile(`${evidence}/${image.asset}.webp`, bytes);
    downloads.push({file:image.file, sourceSha256:image.sha256, url:asset.url, hostedSha256:sha256(bytes)});
  }
  assert.deepEqual((await api('/gameinfo/clash/1')).gallery, before.gallery, 'Concurrent gallery edit');
  await api('/gameinfo/clash/gallery', 'PUT', {gallery});
  const after = await api('/gameinfo/clash/1');
  assert.deepEqual(after.gallery, gallery);
  const unaffected = value => {
    const copy = structuredClone(value);
    delete copy.gallery; delete copy.assets; delete copy.updatedAt;
    return copy;
  };
  assert.deepEqual(unaffected(after), unaffected(before), 'Unrelated game settings changed');
  assert.deepEqual(await api('/gameinfo/clash/meta'), metaBefore, 'General metadata changed');
  for (const [key, asset] of Object.entries(before.assets)) assert.deepEqual(after.assets[key], asset);
  await verifyRules();
  await save('publication-after', after);
  await save('hosted-verification', downloads);
  console.log('Published 3 pairs; six hosted WebP conversions match byte-for-byte. Rules and unrelated metadata unchanged.');
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
