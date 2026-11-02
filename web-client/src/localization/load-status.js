import { languages, resolveLocale } from './runtime.js';

/** Manage asynchronous locale selection and its retry UI independently of the DOM translator. */
export function createLocaleSelection(target, loadLocale, applyLocale) {
  let revision = 0;
  let destroyed = false;
  let ready = Promise.resolve(true);
  let banner;
  let retry;
  function clearStatus() {
    if (retry) retry.onclick = null;
    banner?.remove();
    banner = undefined;
    retry = undefined;
  }
  function showFailure(value) {
    clearStatus();
    const document = target.ownerDocument;
    banner = document.createElement('div');
    banner.setAttribute('data-clash-locale-status', '');
    // This is fallback UI, not source text for the DOM translator.
    banner.setAttribute('translate', 'no');
    banner.setAttribute('lang', 'en');
    banner.setAttribute('dir', 'ltr');
    banner.style.cssText =
      'position:fixed;bottom:12px;left:12px;right:12px;margin:auto;max-width:560px;z-index:10000;padding:10px 14px;border:1px solid #a8a8a8;border-radius:6px;background:#fff;color:#222;box-shadow:0 2px 8px #0003;font:14px/1.4 sans-serif;display:flex;align-items:center;gap:12px;flex-wrap:wrap;';
    const status = document.createElement('span');
    status.setAttribute('role', 'status');
    status.setAttribute('aria-live', 'polite');
    status.textContent = 'Could not load the selected language. Showing English.';
    retry = document.createElement('button');
    retry.type = 'button';
    retry.style.cssText =
      'font:inherit;padding:6px 10px;cursor:pointer;background:#fff;color:#222;border:1px solid #777;border-radius:4px;';
    retry.append(document.createTextNode('Retry language download'));
    const name = document.createElement('bdi');
    name.setAttribute('lang', resolveLocale(value));
    name.textContent = languages[resolveLocale(value)];
    retry.append(document.createTextNode(' — '), name);
    retry.onclick = () => setLocale(value);
    banner.append(status, retry);
    target.append(banner);
  }
  function setLocale(value) {
    if (destroyed) return Promise.resolve(false);
    const attempt = ++revision;
    clearStatus();
    ready = loadLocale(value)
      .then((loaded) => {
        if (attempt !== revision) return false;
        applyLocale(loaded);
        return true;
      })
      .catch((error) => {
        if (attempt !== revision) return false;
        console.warn('Could not load game language', error);
        applyLocale('en');
        showFailure(value);
        return true;
      });
    return ready;
  }
  return {
    setLocale,
    get ready() {
      return ready;
    },
    destroy() {
      destroyed = true;
      revision++;
      clearStatus();
    },
  };
}
