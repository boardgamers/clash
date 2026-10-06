import { uploadViewerFiles } from './viewer-files.mjs';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';
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
const declaredTutorial = JSON.parse(await fs.readFile(path.join(root, 'bgs-tutorial.json'), 'utf8'));
// Preserve admin edits when the same chapter versions are already registered.
const tutorialSignature = (value) => JSON.stringify(value?.chapters?.map(({ id, version }) => [id, version]));
const tutorial =
  tutorialSignature(previous.tutorial) === tutorialSignature(declaredTutorial)
    ? previous.tutorial
    : declaredTutorial;
const enginePath = process.argv.find((arg) => arg.startsWith('--engine='))?.slice('--engine='.length);
const engineBytes = enginePath ? await fs.readFile(path.resolve(enginePath)) : null;
let expectedEngine = previous.engine;
// Undo always stops at information reveals; remove the obsolete lobby option.
const civilizationOption = JSON.parse(
  await fs.readFile(path.join(root, 'bgs-civilization-option.json'), 'utf8'),
);
const lengthOption = JSON.parse(await fs.readFile(path.join(root, 'bgs-length-option.json'), 'utf8'));
const variantOption = JSON.parse(await fs.readFile(path.join(root, 'bgs-variant-option.json'), 'utf8'));
const options = [
  ...(previous.options ?? []).filter(
    (option) => !['undo', 'civilization', 'length', 'variant'].includes(option.name),
  ),
  civilizationOption,
  lengthOption,
  variantOption,
];
const declaredPreferences = JSON.parse(await fs.readFile(path.join(root, 'bgs-preferences.json'), 'utf8'));
const replacedPreferences = new Set([
  'skipRazeCity', // Migrated from account preferences to engine-managed player settings.
  'ui_scale',
  'world_zoom_factor',
  'color_profile',
  ...declaredPreferences.map((pref) => pref.name),
]);
const preferences = [
  ...(previous.preferences ?? []).filter((pref) => !replacedPreferences.has(pref.name)),
  ...declaredPreferences,
];
const declaredSettings = JSON.parse(await fs.readFile(path.join(root, 'bgs-settings.json'), 'utf8'));
const settings = [
  ...(previous.settings ?? []).filter((setting) => !declaredSettings.some((s) => s.name === setting.name)),
  ...declaredSettings,
];
const bytes = await fs.readFile(path.join(root, 'dist/viewer.js'));
if (bytes.length > 25 * 1024 * 1024) throw new Error('Viewer exceeds the BGS upload limit');
const hash = createHash('sha256').update(bytes).digest('hex');
console.log(
  JSON.stringify(
    {
      game: 'clash',
      version: Number(version),
      bytes: bytes.length,
      sha256: hash,
      previousViewer: previous.viewer.url,
      preferences,
      settings,
      options,
      tutorial,
      enginePackage: enginePath ?? null,
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
  const files = await uploadViewerFiles(path.join(root, 'dist'), 'viewer.js', [], (query, body) =>
    api(`${endpoint}/viewer/file?${query}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/octet-stream' },
      body,
    }),
  );
  const uploaded = { url: files.url };
  const viewer = {
    ...previous.viewer,
    url: uploaded.url,
    scriptBytes: files.scriptBytes,
    topLevelVariable: 'clash3d',
    dependencies: { scripts: [], stylesheets: [] },
    fullScreen: true,
    fullScreenMobile: true,
    replayable: true,
    thumbnail: true,
    chat: true,
  };
  const current = await api(endpoint);
  if (
    !isDeepStrictEqual(current.viewer, previous.viewer) ||
    !isDeepStrictEqual(current.preferences, previous.preferences) ||
    !isDeepStrictEqual(current.settings, previous.settings) ||
    !isDeepStrictEqual(current.options, previous.options) ||
    !isDeepStrictEqual(current.tutorial, previous.tutorial) ||
    !isDeepStrictEqual(current.engine, previous.engine)
  )
    throw new Error('The viewer or preferences changed during upload; version metadata was not changed');
  if (engineBytes) {
    const uploadedEngine = await api(`${endpoint}/engine`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/octet-stream' },
      body: engineBytes,
    });
    expectedEngine = uploadedEngine.engine;
    const hosted = await fetch(expectedEngine.package.url, { signal: AbortSignal.timeout(60000) });
    if (!hosted.ok || !Buffer.from(await hosted.arrayBuffer()).equals(engineBytes))
      throw new Error('Hosted engine bytes do not match the release');
  }
  await api(endpoint, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ viewer, preferences, settings, options, tutorial }),
  });
  const saved = await api(endpoint);
  if (
    saved.viewer.url !== uploaded.url ||
    saved.viewer.topLevelVariable !== 'clash3d' ||
    !isDeepStrictEqual(saved.preferences, preferences) ||
    !isDeepStrictEqual(saved.settings, settings) ||
    !isDeepStrictEqual(saved.options, options) ||
    !isDeepStrictEqual(saved.tutorial, tutorial) ||
    !isDeepStrictEqual(saved.engine, expectedEngine)
  )
    throw new Error('Published version verification failed; inspect the saved backup');
  console.log(
    JSON.stringify(
      { published: uploaded.url, backup, engineUnchanged: !engineBytes, engine: saved.engine },
      null,
      2,
    ),
  );
}
