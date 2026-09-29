import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

export async function uploadViewerFiles(directory, entry, styles, upload) {
  const files = [];
  async function walk(directory, prefix = '') {
    for (const item of await readdir(directory, { withFileTypes: true })) {
      const name = prefix + item.name;
      assert(!item.isSymbolicLink(), `Symlink in viewer build: ${name}`);
      if (item.isDirectory()) await walk(join(directory, item.name), name + '/');
      else if (
        /\.(js|css|json|wasm|svg|png|jpe?g|webp|avif|gif|woff2?|ttf|mp3|ogg|wav)$/.test(name) &&
        !/^(engine|preview)\.(js|css)$/.test(name)
      ) {
        files.push({ name, bytes: await readFile(join(directory, item.name)) });
      }
    }
  }
  await walk(directory);
  files.sort((a, b) => a.name.localeCompare(b.name));
  for (const name of [entry, ...styles])
    assert(
      files.some((file) => file.name === name),
      `Missing viewer asset: ${name}`,
    );
  const hash = createHash('sha256');
  for (const file of files) hash.update(file.name).update('\0').update(file.bytes).update('\0');
  const bundle = 'assets-' + hash.digest('hex').slice(0, 20);
  const urls = new Map();
  for (const file of files) {
    const saved = await upload(`filename=${encodeURIComponent(file.name)}&bundle=${bundle}`, file.bytes);
    const response = await fetch(saved.url, { signal: AbortSignal.timeout(60000) });
    assert(response.ok, `Viewer asset unavailable: ${file.name}`);
    assert(
      Buffer.from(await response.arrayBuffer()).equals(file.bytes),
      `Viewer asset mismatch: ${file.name}`,
    );
    urls.set(file.name, saved.url);
  }
  return {
    url: urls.get(entry),
    scriptBytes: files.find((file) => file.name === entry).bytes.length,
    stylesheets: styles.map((name) => urls.get(name)),
  };
}
