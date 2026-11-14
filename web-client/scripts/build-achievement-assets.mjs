import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { chromium } from 'playwright';
import { createServer as createHttpServer } from 'node:http';
import {
  webRoot,
  root,
  outputDir,
  manifestPath,
  sha256,
  validateInputs,
  validateOutputs,
  pngDimensions,
} from './achievement-asset-inputs.mjs';
import assert from 'node:assert/strict';

const manifest = await validateInputs();
if (process.argv.includes('--check')) {
  await validateOutputs();
  console.log('Achievement asset inputs and generated receipt verified.');
} else {
  let server, browser, httpServer;
  try {
    server = await createServer({
      configFile: false,
      appType: 'custom',
      root: webRoot,
      publicDir: false,
      plugins: [
        svelte(),
        {
          name: 'achievement-svelte-runtime',
          resolveId(id) {
            if (id === '/achievement-runtime.js') return '\0achievement-runtime';
          },
          load(id) {
            if (id === '\0achievement-runtime') return "export { mount, unmount, flushSync } from 'svelte';";
          },
        },
      ],
      server: {
        middlewareMode: true,
        hmr: false,
        watch: { ignored: ['**/.engine/**'] },
        fs: { allow: [root] },
      },
      cacheDir: path.join(webRoot, '.engine/achievement-assets-vite'),
    });
    server.middlewares.use('/achievement-render', (_req, res) => {
      res.setHeader('Content-Type', 'text/html');
      res.end('<!doctype html><html><body></body></html>');
    });
    httpServer = createHttpServer(server.middlewares);
    await new Promise((resolve) => httpServer.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${httpServer.address().port}`;
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 1440, height: 1100 }, deviceScaleFactor: 1 });
    await page.route('**/*', (route) =>
      new URL(route.request().url()).origin === origin ? route.continue() : route.abort(),
    );
    await page.goto(`${origin}/achievement-render`);
    // Use the actual canonical Svelte component, serializing SVG exactly as the
    // map flags do. PNGs and SVGs both use their alpha as source-in ink masks.
    const render = async () =>
      page.evaluate(async (manifest) => {
        const { civilizationImages, civilizationIcon } = await import('/src/civilization-emblems.ts');
        const { mount, unmount, flushSync } = await import('/achievement-runtime.js');
        const results = [];
        for (const asset of manifest.assets) {
          // validateInputs has already matched every name and source to the
          // canonical registry; this loop cannot request the fallback icon.
          let source = civilizationImages[asset.civilization];
          if (asset.source.type === 'lucide') {
            if (source) throw new Error('Unexpected canonical PNG');
            const target = document.createElement('div');
            const icon = mount(civilizationIcon(asset.civilization), {
              target,
              props: { size: 128, color: '#000', strokeWidth: manifest.render.strokeWidth },
            });
            flushSync();
            const svg = target.querySelector('svg');
            if (!svg) throw new Error('Missing canonical SVG');
            source = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;
            await unmount(icon);
          }
          if (!source) throw new Error('Missing canonical emblem');
          const image = new Image();
          image.src = source;
          await image.decode();
          const size = manifest.render.emblemSize;
          const mask = document.createElement('canvas');
          mask.width = mask.height = size;
          const ink = mask.getContext('2d');
          const scale = size / Math.max(image.naturalWidth, image.naturalHeight);
          const w = image.naturalWidth * scale,
            h = image.naturalHeight * scale;
          ink.drawImage(image, (size - w) / 2, (size - h) / 2, w, h);
          ink.globalCompositeOperation = 'source-in';
          ink.fillStyle = manifest.render.ink;
          ink.fillRect(0, 0, size, size);
          const canvas = document.createElement('canvas');
          canvas.width = manifest.width;
          canvas.height = manifest.height;
          const ctx = canvas.getContext('2d');
          ctx.fillStyle = manifest.render.background;
          ctx.fillRect(0, 0, canvas.width, canvas.height);
          ctx.drawImage(mask, (canvas.width - size) / 2, (canvas.height - size) / 2);
          results.push({ key: asset.key, data: canvas.toDataURL('image/png').split(',')[1] });
        }
        return results;
      }, manifest);
    const first = await render();
    // Independent reload ensures no retained component/canvas state affects bytes.
    await page.reload();
    assert.deepEqual(await render(), first, 'PNG generation must be byte-reproducible');
    await mkdir(outputDir, { recursive: true });
    const assets = [];
    for (let i = 0; i < first.length; i++) {
      const bytes = Buffer.from(first[i].data, 'base64');
      const dimensions = pngDimensions(bytes);
      assert.deepEqual(dimensions, { width: 256, height: 256 });
      const asset = manifest.assets[i];
      const file = `${asset.key}.png`;
      await writeFile(path.join(outputDir, file), bytes);
      assets.push({
        civilization: asset.civilization,
        key: asset.key,
        file,
        sha256: sha256(bytes),
        ...dimensions,
      });
    }
    const receipt = {
      revision: 2,
      manifestSha256: sha256(await readFile(manifestPath)),
      width: 256,
      height: 256,
      renderer: {
        chromium: browser.version(),
        platform: process.platform,
        arch: process.arch,
        reproducibility: 'two independent page renders byte-identical',
      },
      assets,
    };
    await writeFile(path.join(outputDir, 'receipt.json'), `${JSON.stringify(receipt, null, 2)}\n`);
    const cards = assets
      .map(
        (a, i) =>
          `<figure><img src="data:image/png;base64,${first[i].data}"><figcaption>${a.civilization}<br><small>${a.key}</small></figcaption></figure>`,
      )
      .join('');
    const html = `<!doctype html><meta charset="utf-8"><title>Achievement artwork review — local only</title><style>body{font:16px sans-serif;margin:24px;background:#fff;color:#18212b}section{display:grid;grid-template-columns:repeat(5,1fr);gap:16px;padding:16px}figure{margin:0;text-align:center}img{width:192px;height:192px}figcaption{padding:8px}small{font-size:11px}.dark{background:#18212b;color:#fff}</style><h1>Local achievement artwork review</h1><p>15 canonical symbols. Established project PNG reuse; per-item licenses unverified. Final review required.</p><section>${cards}</section><h2>Dark surround</h2><section class="dark">${cards}</section>`;
    await writeFile(path.join(outputDir, 'contact-sheet.html'), html);
    await page.setContent(html);
    await page.screenshot({ path: path.join(outputDir, 'contact-sheet.png'), fullPage: true });
    await validateOutputs();
    console.log(`Generated and verified 15 badges + contact-sheet.html/png + receipt.json in ${outputDir}`);
  } finally {
    try {
      await browser?.close();
    } finally {
      try {
        await server?.close();
      } finally {
        if (httpServer)
          await new Promise((resolve, reject) =>
            httpServer.close((error) => (error ? reject(error) : resolve())),
          );
      }
    }
  }
}
