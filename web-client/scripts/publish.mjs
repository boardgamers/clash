import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const version = process.argv.find((a) => a.startsWith('--version='))?.split('=')[1] ?? '1';
if (!/^\d+$/.test(version)) throw new Error('Use --version=<integer>');
const token =
  process.env.BGS_ADMIN_TOKEN ?? (await fs.readFile(path.join(os.homedir(), '.bgs'), 'utf8')).trim();
const endpoint = `https://admin.boardgamers.space/api/admin/gameinfo/clash/${version}`;
async function api(url, options = {}) {
  const res = await fetch(url, {
    ...options,
    signal: AbortSignal.timeout(60000),
    redirect: 'error',
    headers: { Authorization: `Bearer ${token}`, ...options.headers },
  });
  if (!res.ok) throw new Error(`BGS API ${res.status}: ${await res.text()}`);
  return res.json();
}
const previous = await api(endpoint);
const declaredPreferences = JSON.parse(await fs.readFile(path.join(root, 'bgs-preferences.json'), 'utf8'));
const replacedPreferences = new Set([
  'ui_scale',
  'world_zoom_factor',
  'color_profile',
  ...declaredPreferences.map((pref) => pref.name),
]);
const preferences = [
  ...(previous.preferences ?? []).filter((pref) => !replacedPreferences.has(pref.name)),
  ...declaredPreferences,
];
const bytes = await fs.readFile(path.join(root, 'dist/viewer.js'));
if (bytes.length > 25 * 1024 * 1024) throw new Error('Viewer exceeds the BGS upload limit');
const hash = createHash('sha256').update(bytes).digest('hex');
const filename = `viewer-${hash.slice(0, 16)}.js`;
console.log(
  JSON.stringify(
    {
      game: 'clash',
      version: Number(version),
      bytes: bytes.length,
      sha256: hash,
      previousViewer: previous.viewer.url,
      preferences,
      dryRun: process.argv.includes('--dry-run'),
    },
    null,
    2,
  ),
);
if (!process.argv.includes('--dry-run')) {
  const backupDir = path.join(root, '.engine/releases');
  await fs.mkdir(backupDir, { recursive: true });
  const backup = path.join(backupDir, `version-${version}-${Date.now()}.json`);
  await fs.writeFile(backup, JSON.stringify(previous, null, 2));
  const uploaded = await api(`${endpoint}/viewer/file?filename=${filename}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/octet-stream' },
    body: bytes,
  });
  const asset = await fetch(uploaded.url, { redirect: 'error', signal: AbortSignal.timeout(60000) });
  if (
    !asset.ok ||
    createHash('sha256')
      .update(new Uint8Array(await asset.arrayBuffer()))
      .digest('hex') !== hash
  )
    throw new Error('Uploaded asset verification failed; version metadata was not changed');
  const viewer = {
    ...previous.viewer,
    url: uploaded.url,
    topLevelVariable: 'clash3d',
    dependencies: { scripts: [], stylesheets: [] },
    fullScreen: true,
    fullScreenMobile: true,
    replayable: false,
    chat: true,
  };
  const current = await api(endpoint);
  if (
    JSON.stringify(current.viewer) !== JSON.stringify(previous.viewer) ||
    JSON.stringify(current.preferences) !== JSON.stringify(previous.preferences)
  )
    throw new Error('The viewer or preferences changed during upload; version metadata was not changed');
  await api(endpoint, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ viewer, preferences }),
  });
  const saved = await api(endpoint);
  if (
    saved.viewer.url !== uploaded.url ||
    saved.viewer.topLevelVariable !== 'clash3d' ||
    JSON.stringify(saved.preferences) !== JSON.stringify(preferences) ||
    JSON.stringify(saved.engine) !== JSON.stringify(previous.engine)
  )
    throw new Error('Published version verification failed; inspect the saved backup');
  console.log(JSON.stringify({ published: uploaded.url, backup, engineUnchanged: true }, null, 2));
}
