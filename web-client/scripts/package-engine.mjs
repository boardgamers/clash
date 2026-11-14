// Run build:bridge first. Package exactly that tested Node/WASM bridge, not a viewer.
import { copyFile, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import path from 'node:path';

const root = fileURLToPath(new URL('..', import.meta.url));
const metadata = JSON.parse(await readFile(path.join(root, 'release/engine-package.json'), 'utf8'));
const staging = path.join(root, '.engine/releases/package');
const output = path.join(root, '.engine/releases');
await rm(staging, { recursive: true, force: true });
await mkdir(staging, { recursive: true });
for (const file of metadata.files) await copyFile(path.join(root, '.engine', file), path.join(staging, file));
await copyFile(path.join(root, '../LICENSE'), path.join(staging, 'LICENSE'));
await writeFile(path.join(staging, 'package.json'), JSON.stringify(metadata, null, 2) + '\n');
const [packed] = JSON.parse(
  execFileSync('npm', ['pack', '--ignore-scripts', '--json', '--pack-destination', output], {
    cwd: staging,
    encoding: 'utf8',
  }),
);
const artifact = path.join(output, packed.filename);
const sha256 = createHash('sha256')
  .update(await readFile(artifact))
  .digest('hex');
console.log(JSON.stringify({ artifact, name: metadata.name, version: metadata.version, sha256 }, null, 2));
