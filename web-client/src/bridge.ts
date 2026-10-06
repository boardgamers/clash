import initWasm, { webView, webCollectPreview, webRecruitPreview, webQuery } from '../.bridge/server.js';
import wasm from '../.bridge/server_bg.wasm?url&no-inline';
import type { Bridge } from './types';
let ready: Promise<Bridge> | undefined;
export function loadBridge(): Promise<Bridge> {
  return (ready ??= initialize());
}
async function initialize(): Promise<Bridge> {
  const response = await fetch(wasm);
  if (!response.ok) throw new Error(`Could not load the game engine (HTTP ${response.status}).`);
  // The full engine exceeds browsers' synchronous WebAssembly compilation limit.
  await initWasm({ module_or_path: response });
  return { webView, webCollectPreview, webRecruitPreview, webQuery };
}

/** Tutorials run the same initialized engine in an isolated local state. */
export async function loadTutorialEngine() {
  await loadBridge();
  const { tryMove, stripSecret, currentPlayer, webView, webQuery } = await import('../.bridge/server.js');
  return { tryMove, stripSecret, currentPlayer, webView, webQuery };
}
