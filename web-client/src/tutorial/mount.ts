import { mount, unmount, tick } from 'svelte';
import { createTutorial, type TutorialMount, type TutorialSnapshot } from '@boardgamers/protocol/tutorial';
import { mountTutorialGuide } from '@boardgamers/protocol/tutorial/dom';
import { Controller } from '../controller';
import App from '../App.svelte';
import { loadTutorialEngine } from '../bridge';
import { mountLocalization } from '../localization';
import { createLesson, type Action, type State } from './lessons';
import gameCss from '../style.css?inline';
import tutorialCss from './tutorial.css?inline';

const positions = import.meta.glob('./positions/*.json', {
  eager: true,
  query: '?url&no-inline',
  import: 'default',
}) as Record<string, string>;
export const mountTutorial: TutorialMount = async (target, options) => {
  const url = positions[`./positions/${options.chapter}.json`];
  if (!url) throw new Error(`Unknown Clash of Cultures chapter: ${options.chapter}`);
  const response = await fetch(url, { signal: AbortSignal.timeout(20000) });
  if (!response.ok) throw new Error(`Could not load tutorial position (HTTP ${response.status}).`);
  const initialGame = JSON.stringify(await response.json());
  const engine = await loadTutorialEngine();
  const lesson = createLesson(options.chapter, engine, initialGame);
  const localization = mountLocalization(target, options.locale);
  await localization.ready;
  const style = document.createElement('style');
  style.textContent = gameCss + tutorialCss;
  document.head.append(style);
  const layout = document.createElement('div');
  layout.className = 'clash-tutorial';
  const sidebar = document.createElement('aside');
  sidebar.className = 'tutorial-sidebar';
  const guide = document.createElement('section');
  const choices = document.createElement('div');
  choices.className = 'tutorial-choices';
  choices.setAttribute('aria-label', 'Lesson actions');
  const board = document.createElement('div');
  board.className = 'tutorial-board';
  const controlPath = document.createElement('p');
  controlPath.className = 'tutorial-control-path';
  sidebar.append(guide, controlPath, choices);
  layout.append(sidebar, board);
  target.append(layout);
  let storage: Storage | undefined;
  try {
    storage = localStorage;
  } catch {
    /* Saving is optional. */
  }
  const tutorial = await createTutorial({ ...lesson, storage, onProgress: options.onProgress });
  let latest: TutorialSnapshot<State> = tutorial.snapshot;
  let destroyed = false;
  let displayed: string | undefined;
  let queue = Promise.resolve();
  const send = async (action: Action) => {
    if (destroyed || latest.busy || latest.completed) return;
    const accepted = await tutorial.play(action);
    if (!destroyed && !accepted)
      controller.handleError(tutorial.snapshot.error || 'Follow the current lesson step.');
  };
  const preferences: Record<string, unknown> = {
    locale: options.locale,
    sound: false,
    replayAutoplay: false,
    availableOnly: false,
  };
  const noop = () => false;
  const controller = new Controller(
    {
      move: (move) => {
        void send({ step: lesson.steps[latest.step].id, kind: 'move', move: JSON.parse(move) });
        return true;
      },
      openPlayer: noop,
      hoverPlayer: noop,
      leavePlayer: noop,
      openBoardgame: noop,
      updateSetting: noop,
      updatePreference: (name, value) => {
        preferences[name] = value;
        controller.setPreferences(preferences);
        return true;
      },
      fetchState: () => {
        void controller.load(engine.stripSecret(latest.state.game, 0));
        return true;
      },
      fetchLog: noop,
      addLog: () => true,
      replaceLog: () => true,
      setReplayInfo: () => true,
    },
    new URL('.', document.baseURI),
    // Lessons validate each step and answer for scripted opponents; show their results only.
    { predict: false },
  );
  controller.setPlayer(0);
  controller.setPreferences(preferences);
  const app = mount(App, { target: board, props: { controller } });
  const removeGuide = mountTutorialGuide(guide, tutorial, { nextChapter: options.nextChapter });
  const remove = tutorial.subscribe((snapshot) => {
    latest = snapshot;
    choices.replaceChildren();
    controlPath.replaceChildren();
    layout.dataset.lessonControl = snapshot.target ?? '';
    if (!snapshot.completed) {
      const labels = lesson.steps[snapshot.step].controls(snapshot.state);
      labels.forEach((label, index) => {
        if (index) controlPath.append(document.createTextNode(' → '));
        const part = document.createElement('span');
        part.textContent = label;
        controlPath.append(part);
      });
    }
    controlPath.hidden = !controlPath.childNodes.length;
    if (!snapshot.completed)
      for (const offer of lesson.steps[snapshot.step]
        .offers(snapshot.state)
        .filter((offer) => offer.action.kind === 'answer')) {
        const button = document.createElement('button');
        button.type = 'button';
        button.textContent = offer.label;
        button.disabled = snapshot.busy;
        button.onclick = () => void send(offer.action);
        choices.append(button);
      }
    if (snapshot.state.game !== displayed) {
      displayed = snapshot.state.game;
      const raw = engine.stripSecret(snapshot.state.game, 0);
      queue = queue
        .then(async () => {
          if (!destroyed) {
            await controller.load(raw);
            await tick();
            localization.refresh();
          }
        })
        .catch((error) => {
          if (!destroyed) controller.handleError(error);
        });
    }
    localization.refresh();
  });
  await queue;
  return () => {
    destroyed = true;
    remove();
    removeGuide();
    tutorial.destroy();
    controller.destroy();
    localization.destroy();
    void unmount(app);
    style.remove();
    layout.remove();
  };
};
