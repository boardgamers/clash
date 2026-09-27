import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { previewApi } from './scripts/preview-api';
export default defineConfig({
  plugins: [svelte(), previewApi()],
  build: {
    lib: { entry: 'src/viewer.ts', name: 'Clash3DBundle', formats: ['iife'], fileName: () => 'viewer.js' },
    target: 'es2022',
    emptyOutDir: true,
  },
  server: { strictPort: true },
});
