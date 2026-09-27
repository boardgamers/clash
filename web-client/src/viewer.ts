import { mount, unmount, tick } from 'svelte';
import { registerViewer } from '@boardgamers/protocol/viewer';
import App from './App.svelte';
import { Controller } from './controller';
import css from './style.css?inline';
const source = document.currentScript?.getAttribute('src');
const assetBase = new URL(source ? '.' : '/', source ? new URL(source, location.href) : location.href);
export const viewer = registerViewer<string, string>('clash3d', ({ target, ...commands }) => {
  const style = document.createElement('style');
  style.textContent = css;
  document.head.append(style);
  const controller = new Controller(commands, assetBase);
  const app = mount(App, { target, props: { controller } });
  const motion = matchMedia('(prefers-reduced-motion: reduce)');
  controller.patch({ reducedMotion: motion.matches });
  return {
    chat: controller.chat,
    async onState(state) {
      await controller.load(state);
      await tick();
    },
    onPlayer({ index }) {
      controller.setPlayer(index);
    },
    onAvatars(avatars) {
      controller.patch({ avatars });
    },
    onPreferences(prefs) {
      controller.patch({
        locale: typeof prefs.locale === 'string' ? prefs.locale : 'en',
        colorBlind: prefs.colorBlind === true,
      });
    },
    onTheme({ dark }) {
      controller.patch({ dark });
    },
    onError(error) {
      controller.patch({ error: String(error), pending: false });
    },
    destroy() {
      controller.destroy();
      void unmount(app);
      style.remove();
    },
  };
});
