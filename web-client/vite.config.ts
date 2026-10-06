import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { previewApi } from './scripts/preview-api';
export default defineConfig({
  base: './',
  publicDir: false,
  plugins: [
    {
      name: 'localization-font-licenses',
      generateBundle() {
        const licenses = Object.fromEntries(
          ['LICENSE-Noto.txt', 'LICENSE-Noto-Arabic.txt'].map((name) => [
            name,
            readFileSync(new URL(`./src/localization/fonts/${name}`, import.meta.url), 'utf8'),
          ]),
        );
        this.emitFile({
          type: 'asset',
          fileName: 'font-licenses.json',
          source: JSON.stringify(licenses, null, 2),
        });
      },
    },
    {
      name: 'external-wasm-fallback',
      enforce: 'pre',
      transform(code, id) {
        if (!id.endsWith('/.bridge/server.js')) return;
        // wasm-bindgen's default URL must stay external in library builds too.
        // Otherwise Vite embeds a second copy alongside our explicit Wasm import.
        return code.replace(
          "new URL('server_bg.wasm', import.meta.url)",
          "new URL('server_bg.wasm?no-inline', import.meta.url)",
        );
      },
    },
    svelte(),
    previewApi(),
  ],
  build: {
    lib: { entry: 'src/viewer.ts', name: 'Clash3DBundle', formats: ['iife'], fileName: () => 'viewer.js' },
    target: 'es2022',
    emptyOutDir: true,
  },
  server: { strictPort: true },
});
