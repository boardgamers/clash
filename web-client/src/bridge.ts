import { initSync, webView, webCollectPreview, webRecruitPreview } from '../.bridge/server.js';
import wasm from '../.bridge/server_bg.wasm?url';
import type { Bridge } from './types';
let ready: Promise<Bridge> | undefined;
export function loadBridge(): Promise<Bridge> {
  return (ready ??= initialize());
}
async function initialize(): Promise<Bridge> {
  const bytes = wasm.startsWith('data:')
    ? Uint8Array.from(atob(wasm.split(',')[1]), (c) => c.charCodeAt(0))
    : new Uint8Array(await (await fetch(wasm)).arrayBuffer());
  initSync({ module: bytes });
  return { webView, webCollectPreview, webRecruitPreview };
}
