import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { fileURLToPath } from 'node:url';

// Metadata-only update: no engine or viewer upload is needed.
const root = fileURLToPath(new URL('..', import.meta.url));
const version = process.argv.find((arg) => arg.startsWith('--version='))?.split('=')[1] ?? '1';
if (!/^\d+$/.test(version)) throw new Error('Use --version=<integer>');
const declared = JSON.parse(await fs.readFile(path.join(root, 'bgs-reference-links.json'), 'utf8'));
const token = process.env.BGS_ADMIN_TOKEN ?? (await fs.readFile(path.join(os.homedir(), '.bgs'), 'utf8')).trim();
const endpoint = `https://admin.boardgamers.space/api/admin/gameinfo/clash/${version}`;
async function api(options = {}) {
  const response = await fetch(endpoint, {
    ...options,
    redirect: 'error',
    signal: AbortSignal.timeout(60000),
    headers: { Authorization: `Bearer ${token}`, ...options.headers },
  });
  if (!response.ok) throw new Error(`BGS API ${response.status}`);
  return response.json();
}
const previous = await api();
const patch = { rules: declared.rules, links: { ...previous.links, ...declared.links } };
if (process.argv.includes('--dry-run')) {
  console.log(JSON.stringify(patch, null, 2));
} else {
  const backupDir = path.join(root, '.engine/releases');
  await fs.mkdir(backupDir, { recursive: true });
  await fs.writeFile(path.join(backupDir, `reference-links-${version}-${Date.now()}.json`), JSON.stringify(previous, null, 2));
  const current = await api();
  if (!isDeepStrictEqual(current.rules, previous.rules) || !isDeepStrictEqual(current.links, previous.links)) {
    throw new Error('Reference links changed during preparation; no update sent');
  }
  await api({ method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(patch) });
  const saved = await api();
  for (const key of ['rules', 'links', 'engine', 'viewer']) {
    if (!isDeepStrictEqual(saved[key], key in patch ? patch[key] : previous[key])) {
      throw new Error(`Verification failed for ${key}`);
    }
  }
  console.log(JSON.stringify({ updated: true, ...patch }, null, 2));
}
