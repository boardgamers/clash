import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { chromium } from 'playwright';
import { ids, svg } from './nonfaction-art.mjs';
import { root, sha256, pngDimensions } from './achievement-asset-inputs.mjs';
export const outputDir = path.join(root, 'web-client/.engine/achievement-assets-nonfaction');
export const manifestPath = path.join(root, 'web-client/release/achievement-assets-nonfaction.json');
export async function validateInputs() {
  const manifest = JSON.parse(await readFile(manifestPath));
  assert.equal(manifest.revision, 3);
  assert.deepEqual(
    manifest.assets.map((a) => a.key),
    ids.map((id) => `achievement-${id}`),
  );
  assert.equal(manifest.assets.length, 26);
  for (const a of manifest.assets) {
    assert.equal(a.source.type, 'original-svg');
    assert.equal(a.source.path, 'web-client/scripts/nonfaction-art.mjs');
    assert.equal(a.source.sha256, sha256(await readFile(path.join(root, a.source.path))));
  }
  return manifest;
}
export async function validateOutputs() {
  const manifest = await validateInputs();
  const receipt = JSON.parse(await readFile(path.join(outputDir, 'receipt.json')));
  assert.equal(receipt.revision, 3);
  assert.equal(receipt.manifestSha256, sha256(await readFile(manifestPath)));
  assert.deepEqual(
    receipt.assets.map((a) => a.key),
    manifest.assets.map((a) => a.key),
  );
  assert.equal(new Set(receipt.assets.map((a) => a.sha256)).size, 26);
  for (const a of receipt.assets) {
    assert.equal(a.file, `${a.key}.png`);
    const b = await readFile(path.join(outputDir, a.file));
    assert.equal(sha256(b), a.sha256);
    assert.deepEqual(pngDimensions(b), { width: 256, height: 256 });
  }
  return receipt;
}
export async function build() {
  if (process.argv.includes('--check')) return validateOutputs();
  await mkdir(outputDir, { recursive: true });
  const source = {
    type: 'original-svg',
    path: 'web-client/scripts/nonfaction-art.mjs',
    sha256: sha256(await readFile(new URL('./nonfaction-art.mjs', import.meta.url))),
    provenance: {
      type: 'original-geometry',
      license: 'MIT',
      author: 'Clash project contributors',
      evidence: 'New procedural geometry; no reused external image or traced input. Repository LICENSE.',
    },
  };
  const manifest = {
    revision: 3,
    width: 256,
    height: 256,
    assets: ids.map((id) => ({ key: `achievement-${id}`, source })),
  };
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    const assets = [];
    for (const id of ids) {
      const key = `achievement-${id}`,
        vector = svg(id);
      await writeFile(path.join(outputDir, `${key}.svg`), vector);
      const render = () =>
        page.evaluate(async (vector) => {
          const i = new Image();
          i.src = 'data:image/svg+xml;base64,' + btoa(vector);
          await i.decode();
          const c = document.createElement('canvas');
          c.width = c.height = 256;
          c.getContext('2d').drawImage(i, 0, 0);
          return c.toDataURL().split(',')[1];
        }, vector);
      const a = await render();
      assert.equal(await render(), a);
      const bytes = Buffer.from(a, 'base64');
      await writeFile(path.join(outputDir, `${key}.png`), bytes);
      assets.push({ key, file: `${key}.png`, sha256: sha256(bytes), width: 256, height: 256 });
    }
    const receipt = {
      revision: 3,
      manifestSha256: sha256(await readFile(manifestPath)),
      renderer: browser.version(),
      assets,
    };
    await writeFile(path.join(outputDir, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n');
    const cards = ids
      .map(
        (id) =>
          `<figure><figcaption>${id}</figcaption>${[32, 48, 64].map((size) => `<img width="${size}" height="${size}" src="data:image/svg+xml;base64,${Buffer.from(svg(id)).toString('base64')}">`).join('')}</figure>`,
      )
      .join('');
    const html = `<!doctype html><title>Clash 26 original achievements</title><style>body{font:13px sans-serif}section{display:grid;grid-template-columns:repeat(4,1fr);gap:8px;padding:16px}figure{margin:0;padding:10px}img{margin:5px;border-radius:50%}.dark{background:#101722;color:white}</style><h1>Clash — 32 / 48 / 64px circular review</h1><section>${cards}</section><section class="dark">${cards}</section>`;
    await writeFile(path.join(outputDir, 'contact-sheet.html'), html);
    await page.setViewportSize({ width: 1100, height: 900 });
    await page.setContent(html);
    await page.screenshot({ path: path.join(outputDir, 'contact-sheet.png'), fullPage: true });
    await validateOutputs();
  } finally {
    await browser.close();
  }
}
