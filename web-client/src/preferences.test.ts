import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createViewer } from '@boardgamers/protocol/viewer';
import { readPreferences } from './preferences.ts';
import { GameAudio } from './audio.ts';

function audioDevice() {
  const parameter = () => ({
    value: 0,
    setValueAtTime(value: number) {
      this.value = value;
    },
    linearRampToValueAtTime() {},
    exponentialRampToValueAtTime() {},
  });
  const gains: any[] = [];
  const voices: any[] = [];
  const device = {
    state: 'suspended',
    currentTime: 10,
    destination: {},
    closed: false,
    createGain() {
      const gain = { gain: parameter(), connect() {}, disconnect() {} };
      gains.push(gain);
      return gain;
    },
    createOscillator() {
      const voice = {
        frequency: parameter(),
        type: '',
        stops: [] as (number | undefined)[],
        onended: null,
        connect() {},
        disconnect() {},
        start() {},
        stop(at?: number) {
          this.stops.push(at);
        },
      };
      voices.push(voice);
      return voice;
    },
    async resume() {
      this.state = 'running';
    },
    async close() {
      this.closed = true;
      this.state = 'closed';
    },
  };
  return { device, gains, voices };
}

test('live BGS sound preferences silence active and queued notes without replaying them on unmute', () => {
  const { device, gains, voices } = audioDevice();
  let devices = 0;
  const audio = new GameAudio(() => {
    devices++;
    return device as unknown as AudioContext;
  });
  const viewer = createViewer({
    onState() {},
    onPreferences(preferences) {
      audio.setEnabled(readPreferences(preferences).sound);
    },
  });
  audio.unlock();
  audio.play('select');
  assert.equal(devices, 0, 'No audio device before receiving preferences');
  viewer.emitter.receive('preferences', { sound: false });
  audio.unlock();
  assert.equal(devices, 0, 'Muted interactions do not create an audio device');
  viewer.emitter.receive('preferences', { sound: true });
  audio.play('research');
  assert.equal(voices.length, 0, 'No autoplay before a user interaction');
  audio.unlock();
  audio.play('research');
  assert.equal(voices.length, 3);
  viewer.emitter.receive('preferences', { sound: false });
  assert.equal(gains[0].gain.value, 0);
  assert.ok(
    voices.every((voice) => voice.stops.at(-1) === undefined),
    'Stops scheduled notes immediately',
  );
  audio.play('select');
  assert.equal(voices.length, 3);
  viewer.emitter.receive('preferences', { sound: true });
  assert.equal(voices.length, 3, 'Enabling sound does not replay missed effects');
  audio.play('collect');
  assert.equal(voices.length, 5);
  audio.destroy();
  assert.equal(device.closed, true);
  audio.unlock();
  audio.play('select');
  assert.equal(devices, 1);
  assert.equal(voices.length, 5);
  viewer.destroy();
});

test('unavailable audio does not interrupt the viewer', () => {
  const audio = new GameAudio(() => {
    throw new Error('Audio unavailable');
  });
  audio.setEnabled(true);
  assert.doesNotThrow(() => {
    audio.unlock();
    audio.play('confirm');
    audio.destroy();
  });
});

test('map view and unit badges round-trip through BGS preferences and incoming preferences never write back', () => {
  let saved = { sound: false, colorBlind: true, mapView: '3d', unitBadges: true };
  let current = readPreferences({});
  assert.equal(current.unitBadges, false);
  let writes = 0;
  const options = {
    onState() {},
    onPreferences(prefs: Record<string, unknown>) {
      current = readPreferences(prefs);
    },
  };
  const viewer = createViewer(options);
  viewer.emitter.on('update:preference', ({ name, value }) => {
    writes++;
    saved = { ...saved, [name]: value };
    viewer.emitter.receive('preferences', saved);
  });
  viewer.emitter.receive('preferences', saved);
  assert.equal(writes, 0);
  viewer.updatePreference('mapView', '2d');
  assert.deepEqual(current, {
    analysis: false,
    locale: 'en',
    playerColors: [],
    playerSymbols: [],
    playerBadges: [],
    sound: false,
    colorBlind: true,
    topDown: true,
    unitBadges: true,
    replayAutoplay: true,
  });
  assert.equal(writes, 1);
  viewer.updatePreference('unitBadges', false);
  assert.equal(current.unitBadges, false);
  assert.equal(writes, 2);
  viewer.destroy();
  const reopened = createViewer(options);
  reopened.emitter.receive('preferences', saved);
  assert.equal(current.topDown, true);
  assert.equal(current.unitBadges, false);
  reopened.emitter.receive('preferences', { ...saved, sound: true, colorBlind: false });
  assert.equal(current.sound, true);
  assert.equal(current.colorBlind, false);
  assert.equal(current.topDown, true);
  reopened.destroy();
});

test('opponent recap defaults to autoplay and restores the player’s manual preference', () => {
  assert.equal(readPreferences({}).replayAutoplay, true);
  assert.equal(readPreferences({ replayAutoplay: false }).replayAutoplay, false);
  assert.equal(readPreferences({ replayAutoplay: true }).replayAutoplay, true);
});

test('tile and control hover sounds are throttled and obey global mute', () => {
  const { device, voices } = audioDevice();
  const audio = new GameAudio(() => device as unknown as AudioContext);
  audio.setEnabled(true);
  audio.unlock();
  audio.play('hover');
  audio.play('hover');
  device.currentTime += 0.03;
  audio.play('hover');
  assert.equal(voices.length, 1);
  device.currentTime += 0.1;
  audio.play('hover');
  assert.equal(voices.length, 2);
  audio.play('draw');
  assert.equal(voices.length, 5);
  audio.setEnabled(false);
  device.currentTime += 1;
  audio.play('hover');
  audio.play('draw');
  assert.equal(voices.length, 5);
  assert.ok(voices.every((voice) => voice.stops.at(-1) === undefined));
  audio.destroy();
});

test('cosmetic preferences validate hex colours and accessibility keeps its own palette', async () => {
  const { playerColor } = await import('./types.ts');
  const { playerColors } = readPreferences({ bgs: { playerColors: ['#da73aa', 'url(x)'] } });
  assert.equal(playerColor(0, false, playerColors), '#da73aa');
  assert.equal(playerColor(0, true, playerColors), playerColor(0, true));
  assert.deepEqual(readPreferences({ bgs: {} }).playerColors, []);
});

test('player badges use host artwork only for visible Supporters and clear when omitted', () => {
  const supporterBadge = { url: 'https://boardgamers.space/badges/new-design.svg', label: 'Supporter' };
  const { playerBadges } = readPreferences({
    bgs: { players: [{ pro: true }, { pro: false }], supporterBadge },
  });
  assert.deepEqual(playerBadges, [supporterBadge, undefined]);
  assert.deepEqual(readPreferences({}).playerBadges, []);
});

test('player symbol preferences update and reset without affecting resource or faction identity', async () => {
  const { playerSymbol } = await import('./types.ts');
  const { playerSymbols } = readPreferences({ bgs: { playerSymbols: ['star', 'hexagon', '<svg>'] } });
  assert.equal(playerSymbol(0, playerSymbols), '★');
  assert.equal(playerSymbol(1, playerSymbols), '⬢');
  assert.equal(playerSymbol(2, playerSymbols), playerSymbol(2));
  assert.deepEqual(readPreferences({}).playerSymbols, []);
  assert.equal(playerSymbol(0, readPreferences({ bgs: { playerSymbols: ['cross'] } }).playerSymbols), '✕');
});
