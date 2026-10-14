import { GameAudio, type SoundCue } from './audio.ts';

const context = new AudioContext();
const audio = new GameAudio(() => context);
audio.setEnabled(true);
const cues: [SoundCue, string, string][] = [
  ['hover', 'Hover', 'Moving over an interactive target'],
  ['select', 'Select', 'Selecting a unit or option'],
  ['collect', 'Collect', 'Collecting resources'],
  ['build', 'Build / recruit', 'Constructing, founding or recruiting'],
  ['research', 'Research', 'Gaining an advance'],
  ['draw', 'Draw card', 'Drawing a card'],
  ['move', 'Move', 'Moving units'],
  ['confirm', 'Confirm', 'Other confirmed actions'],
  ['undo', 'Undo', 'Reversing an action'],
  ['battle', 'Battle rolls', 'Dice rolling in a battle'],
  ['casualty', 'Unit loss', 'Land units or leaders lost in battle'],
  ['sink', 'Ship sinking', 'Ships lost in battle'],
  ['error', 'Error', 'An action could not be completed'],
];
for (const [cue, title, description] of cues) {
  const button = document.createElement('button');
  button.textContent = `▶ ${title}`;
  const detail = document.createElement('span');
  detail.textContent = description;
  button.append(detail);
  button.addEventListener('click', async () => {
    try {
      audio.unlock();
      await context.resume();
      audio.play(cue);
      document.querySelector('#status')!.textContent = `Played: ${title}`;
    } catch {
      document.querySelector('#status')!.textContent = 'Audio could not start. Check your browser’s sound permissions.';
    }
  });
  document.querySelector('#sounds')!.append(button);
}
window.addEventListener('pagehide', () => audio.destroy());
