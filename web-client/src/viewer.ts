import { mount, unmount, tick } from 'svelte';
import { mountLocalization } from './localization';
import { registerViewer } from '@boardgamers/protocol/viewer';
import App from './App.svelte';
import { mountTutorial } from './tutorial/mount';
import { Controller } from './controller';
import css from './style.css?inline';
const source = document.currentScript?.getAttribute('src');
const assetBase = new URL(source ? '.' : '/', source ? new URL(source, location.href) : location.href);
export const viewer = registerViewer<string, string>(
  'clash3d',
  ({ target, ...commands }) => {
    const style = document.createElement('style');
    style.textContent = css;
    document.head.append(style);
    const localization = mountLocalization(target.ownerDocument.body);
    const controller = new Controller(
      {
        ...commands,
        clearReplayInfo() {
          // BGS clears its replay toolbar with null. Protocol 0.9 types only the open
          // range, so use the host's existing close message until it exposes this API.
          if (window.parent !== window) window.parent.postMessage({ type: 'replay:info', data: null }, '*');
        },
      },
      assetBase,
    );
    const stopNames = controller.session.subscribe((session) => {
      localization.setNames(session.view?.players.map((player) => player.name) ?? []);
    });
    const app = mount(App, { target, props: { controller } });
    const unlockAudio = (event: Event) => {
      if (event.isTrusted) controller.audio.unlock();
    };
    const clickAudio = (event: Event) => {
      if (
        event.isTrusted &&
        event.target instanceof Element &&
        event.target.closest('button:not(:disabled)') &&
        !event.target.closest('.city-map-label')
      )
        controller.audio.play('select');
    };
    const hoverAudio = (event: Event) => {
      if (!(event instanceof PointerEvent)) return;
      if (!event.isTrusted || event.pointerType === 'touch' || !(event.target instanceof Element)) return;
      const button = event.target.closest('button:not(:disabled):not(.city-map-label)');
      if (!button || (event.relatedTarget instanceof Node && button.contains(event.relatedTarget))) return;
      controller.audio.play('hover');
    };
    target.addEventListener('pointerdown', unlockAudio, true);
    target.addEventListener('keydown', unlockAudio, true);
    target.addEventListener('click', clickAudio, true);
    target.addEventListener('pointerover', hoverAudio, true);
    const motion = matchMedia('(prefers-reduced-motion: reduce)');
    controller.patch({ reducedMotion: motion.matches });
    return {
      chat: controller.chat,
      async onState(state) {
        await localization.ready;
        await controller.load(state);
        await tick();
      },
      onReplayStart() {
        controller.startPlayback();
      },
      onReplayTo(index) {
        controller.seekPlayback(index);
      },
      onReplayEnd() {
        controller.endPlayback();
      },
      onPlayer({ index }) {
        controller.setPlayer(index);
      },
      onAvatars(avatars) {
        controller.patch({ avatars });
      },
      onSettings(settings) {
        controller.setSettings(settings);
      },
      async onPreferences(prefs) {
        controller.setPreferences(prefs);
        await localization.setLocale(prefs.locale);
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
        target.removeEventListener('pointerover', hoverAudio, true);
        stopNames();
        localization.destroy();
        controller.destroy();
        void unmount(app);
        style.remove();
      },
    };
  },
  { tutorial: mountTutorial },
);
