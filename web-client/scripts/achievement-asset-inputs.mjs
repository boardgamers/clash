import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import assert from 'node:assert/strict';

export const root = fileURLToPath(new URL('../../', import.meta.url));
export const webRoot = path.join(root, 'web-client');
export const outputDir = path.join(webRoot, '.engine/achievement-assets');
export const manifestPath = path.join(webRoot, 'release/achievement-assets.json');
export const sha256 = (data) => createHash('sha256').update(data).digest('hex');
export const read = (file) => readFile(path.join(root, file));
export const settings = {
  background: '#f3f4f6',
  ink: '#18212b',
  emblemSize: 192,
  strokeWidth: 2.5,
  lucideVersion: '0.577.0',
};

// Deliberately strict: changes to the canonical declarations require explicit review,
// never silently accepting civilizationIcon's generic Landmark fallback.
export async function canonicalAssets() {
  const text = (await read('web-client/src/civilization-emblems.ts')).toString();
  const imports = new Map([...text.matchAll(/import (\w+) from '([^']+\.png)';/g)].map((m) => [m[1], m[2]]));
  const icons = new Map(
    [...text.match(/const icons:[\s\S]*?= \{([\s\S]*?)\};/)[1].matchAll(/(\w+): (\w+),/g)].map((m) => [
      m[1],
      m[2],
    ]),
  );
  const images = new Map(
    [
      ...text
        .match(/export const civilizationImages:[\s\S]*?= \{([\s\S]*?)\};/)[1]
        .matchAll(/(\w+): (\w+),/g),
    ].map((m) => [m[1], m[2]]),
  );
  const registry = (await read('server/src/content/civilizations.rs')).toString();
  const slugs = [...registry.matchAll(/^\s+(\w+)::\1\(\),/gm)].map((m) => m[1]);
  assert.equal(slugs.length, 15, 'Playable registry must contain exactly 15 civilizations');
  const names = [...icons.keys(), ...images.keys()].filter((name) => slugs.includes(name.toLowerCase()));
  assert.equal(names.length, 15, 'Canonical emblems must cover every playable civilization');
  return slugs.map((slug) => {
    const civilization = names.find((name) => name.toLowerCase() === slug);
    assert.ok(civilization, `Missing canonical emblem: ${slug}`);
    let source;
    if (images.has(civilization)) {
      const imported = imports.get(images.get(civilization));
      assert.ok(imported, `Missing PNG import: ${civilization}`);
      source = { type: 'png', path: path.posix.normalize(`web-client/src/${imported}`) };
    } else {
      const icon = icons.get(civilization);
      assert.ok(icon && icon !== 'Landmark', `Unknown civilization: ${civilization}`);
      const kebab = icon.replace(/([a-z])([A-Z0-9])/g, '$1-$2').toLowerCase();
      source = {
        type: 'lucide',
        icon,
        path: `web-client/node_modules/lucide-svelte/dist/icons/${kebab}.svelte`,
      };
    }
    return { civilization, key: `achievement-faction-${slug}`, source };
  });
}

export async function validateInputs(manifest) {
  manifest ??= JSON.parse(await readFile(manifestPath, 'utf8'));
  assert.equal(manifest.revision, 2);
  assert.equal(manifest.width, 256);
  assert.equal(manifest.height, 256);
  assert.deepEqual(manifest.render, settings);
  const canonical = await canonicalAssets();
  assert.deepEqual(
    manifest.assets.map(({ civilization, key, source }) => ({
      civilization,
      key,
      source: Object.fromEntries(
        Object.keys(canonical.find((a) => a.key === key)?.source ?? {}).map((k) => [k, source[k]]),
      ),
    })),
    canonical,
  );
  for (const asset of manifest.assets)
    assert.equal(sha256(await read(asset.source.path)), asset.source.sha256, `Source changed: ${asset.key}`);
  for (const [file, hash] of Object.entries(manifest.integrity))
    assert.equal(sha256(await read(file)), hash, `Input changed: ${file}`);
  const pkg = JSON.parse(await read('web-client/node_modules/lucide-svelte/package.json'));
  assert.equal(pkg.version, settings.lucideVersion);
  assert.equal(
    (await read('web-client/release/achievement-assets/LICENSE-lucide.txt')).toString(),
    (await read('web-client/node_modules/lucide-svelte/LICENSE')).toString(),
  );
  return manifest;
}

export function pngDimensions(bytes) {
  assert.equal(bytes.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

export async function validateOutputs(directory = outputDir) {
  const manifest = await validateInputs();
  const receipt = JSON.parse(await readFile(path.join(directory, 'receipt.json'), 'utf8'));
  assert.equal(receipt.revision, 2);
  assert.equal(receipt.manifestSha256, sha256(await readFile(manifestPath)));
  assert.deepEqual(
    receipt.assets.map(({ civilization, key }) => ({ civilization, key })),
    manifest.assets.map(({ civilization, key }) => ({ civilization, key })),
  );
  for (const asset of receipt.assets) {
    assert.equal(asset.file, `${asset.key}.png`);
    const bytes = await readFile(path.join(directory, asset.file));
    assert.equal(sha256(bytes), asset.sha256);
    assert.deepEqual(pngDimensions(bytes), { width: 256, height: 256 });
    assert.equal(asset.width, 256);
    assert.equal(asset.height, 256);
  }
  return receipt;
}
