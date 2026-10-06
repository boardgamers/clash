import { resolveLocale } from './runtime.js';

/** Keep a URL manifest in the viewer, and fetch only the selected catalog. */
/** @param {Record<string, string>} urls
 * @param {(url: string, options: {signal: AbortSignal}) => Promise<{ok: boolean, status?: number, json?: () => Promise<unknown>}>} fetcher
 */
export function createCatalogLoader(urls, fetcher = (...args) => fetch(...args)) {
  /** @type {Record<string, Record<string, string>>} */
  const catalogs = { en: {} };
  const pending = new Map();
  function load(value) {
    const locale = resolveLocale(value);
    if (catalogs[locale]) return Promise.resolve(locale);
    if (!pending.has(locale)) {
      const request = (async () => {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 20000);
        try {
          const response = await fetcher(urls[`./${locale}.json`], { signal: controller.signal });
          if (!response.ok) throw new Error(`Language ${locale}: HTTP ${response.status}`);
          const catalog = await response.json();
          if (
            !catalog ||
            typeof catalog !== 'object' ||
            Array.isArray(catalog) ||
            Object.values(catalog).some((value) => typeof value !== 'string')
          ) {
            throw new Error(`Invalid language catalog: ${locale}`);
          }
          catalogs[locale] = catalog;
          return locale;
        } finally {
          clearTimeout(timeout);
        }
      })().catch((error) => {
        pending.delete(locale);
        throw error;
      });
      pending.set(locale, request);
    }
    return pending.get(locale);
  }
  return { catalogs, load };
}
