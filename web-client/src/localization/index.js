import localeCss from './rtl.css?inline';
import arabicFont from './fonts/Arabic.woff2?url&no-inline';
import devanagariFont from './fonts/Devanagari.woff2?url&no-inline';
import koreanFont from './fonts/Korean.woff2?url&no-inline';
import chineseFont from './fonts/Chinese.woff2?url&no-inline';
const fontFaces = [
  ['Arabic', arabicFont],
  ['Devanagari', devanagariFont],
  ['Korean', koreanFont],
  ['Chinese', chineseFont],
]
  .map(
    ([name, url]) =>
      `@font-face{font-family:"Clash ${name}";src:url(${JSON.stringify(url)}) format("woff2");font-display:swap}`,
  )
  .join('\n');
import { writable } from 'svelte/store';
export const currentLocale = writable('en');
import { createCatalogLoader } from './loader.js';
import { createTranslator, mountLocalization as mount, resolveLocale } from './runtime.js';
export { languages, resolveLocale } from './runtime.js';

const catalogUrls = import.meta.glob(['./*.json', '!./en.json'], {
  eager: true,
  query: '?url&no-inline',
  import: 'default',
});
const loader = createCatalogLoader(catalogUrls);
export const catalogs = loader.catalogs;
const translators = new Map();
export function loadLocale(value) {
  return loader.load(value);
}
export function translateText(text, locale = 'en') {
  if (!translators.has(locale)) {
    translators.set(locale, createTranslator(catalogs, locale));
  }
  return translators.get(locale).translate(text);
}
export function mountLocalization(target, locale) {
  const style = target.ownerDocument.createElement('style');
  style.textContent = localeCss + fontFaces;
  target.ownerDocument.head.append(style);
  const localization = mount(target, catalogs, 'en');
  let revision = 0;
  let ready = Promise.resolve(true);
  function setLocale(value) {
    const attempt = ++revision;
    ready = loadLocale(value)
      .then((loaded) => {
        if (attempt !== revision) return false;
        localization.setLocale(loaded);
        currentLocale.set(loaded);
        return true;
      })
      .catch((error) => {
        console.warn('Could not load game language', error);
        if (attempt !== revision) return false;
        localization.setLocale('en');
        currentLocale.set('en');
        return true;
      });
    return ready;
  }
  setLocale(locale ?? target.ownerDocument.documentElement.lang ?? 'en');
  return {
    ...localization,
    setLocale,
    get locale() {
      return localization.locale;
    },
    get ready() {
      return ready;
    },
    destroy() {
      revision++;
      localization.destroy();
      style.remove();
    },
  };
}
export function localizeTutorial(mountTutorial) {
  return async (target, options) => {
    const localization = mountLocalization(target, options.locale);
    try {
      await localization.ready;
      const dispose = await mountTutorial(target, options);
      localization.refresh();
      return () => {
        localization.destroy();
        dispose?.();
      };
    } catch (error) {
      localization.destroy();
      throw error;
    }
  };
}
