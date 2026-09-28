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
  const unlockAudio = (event: Event) => {
    if (event.isTrusted) controller.audio.unlock();
  };
  const clickAudio = (event: Event) => {
    if (event.isTrusted && event.target instanceof Element && event.target.closest('button:not(:disabled)'))
      controller.audio.play('select');
  };
  target.addEventListener('pointerdown', unlockAudio, true);
  target.addEventListener('keydown', unlockAudio, true);
  target.addEventListener('click', clickAudio, true);
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
      controller.setPreferences(prefs);
    },
    onTheme({ dark }) {
      controller.patch({ dark });
    },
    onError(error) {
      controller.handleError(error);
    },
    destroy() {
      target.removeEventListener('pointerdown', unlockAudio, true);
      target.removeEventListener('keydown', unlockAudio, true);
      target.removeEventListener('click', clickAudio, true);
      controller.destroy();
      void unmount(app);
      style.remove();
    },
  };
});
