const frame = document.querySelector('iframe')!;
if (new URLSearchParams(location.search).has('built')) frame.src = '/built-viewer.html';
const seatSelect = document.querySelector<HTMLSelectElement>('#seat')!;
let revision = -1;
let seat: number | undefined;
let ready = false;
let dark = false;
let lastMessages = '';
let fetching = false;
let queued = false;
let journal: string[] = [];
const send = (event: string, payload?: unknown) =>
  frame.contentWindow?.postMessage({ source: 'clash-host', event, payload }, location.origin);
const preferences: Record<string, unknown> = {
  locale: new URLSearchParams(location.search).get('locale') ?? 'en',
  sound: true,
  colorBlind: false,
  mapView: '3d',
  homeAtBottom: false,
  unitBadges: false,
  availableOnly: false,
  replayAutoplay: true,
  confirmMoves: true,
};
try {
  const saved = JSON.parse(localStorage.getItem('clash-preview-preferences') ?? '{}');
  for (const key of [
    'sound',
    'colorBlind',
    'homeAtBottom',
    'unitBadges',
    'availableOnly',
    'replayAutoplay',
    'confirmMoves',
    'skipRazeCity',
  ])
    if (typeof saved[key] === 'boolean') preferences[key] = saved[key];
  if (['2d', 'strategy'].includes(saved.mapView)) preferences.mapView = 'strategy';
} catch {}
function savePreference(name: string, value: unknown) {
  if (!(
    ([
      'sound',
      'colorBlind',
      'homeAtBottom',
      'unitBadges',
      'availableOnly',
      'replayAutoplay',
      'confirmMoves',
    ].includes(name) &&
      typeof value === 'boolean') ||
    (name === 'mapView' && (value === '3d' || value === 'strategy'))
  ))
    return;
  preferences[name] = value;
  try {
    localStorage.setItem('clash-preview-preferences', JSON.stringify(preferences));
  } catch {}
  send('preferences', preferences);
}
// BGS shows account preferences outside the viewer; the preview bar stands in for them.
const confirmMoves = document.querySelector<HTMLInputElement>('#confirm-moves')!;
confirmMoves.checked = preferences.confirmMoves !== false;
confirmMoves.onchange = () => savePreference('confirmMoves', confirmMoves.checked);
function showError(message: string) {
  document.querySelector('#host-error')!.textContent = message;
}
async function refresh(force = false) {
  if (fetching) {
    queued = true;
    return;
  }
  fetching = true;
  try {
    const response = await fetch(`/api/state?seat=${seatSelect.value}`);
    const data = await response.json();
    if (!response.ok) throw Error(data.error);
    if (force || data.revision !== revision || data.seat !== seat) {
      revision = data.revision;
      seat = data.seat;
      send('player', { index: seat });
      send('state', data.state);
      send('settings', data.settings);
    }
    send('chat:state', {
      canSend: seat !== undefined,
      reason: seat === undefined ? 'not-a-player' : undefined,
      mentions: [
        { id: '0', name: 'Leif', playerIndex: 0 },
        { id: '1', name: 'Aurelia', playerIndex: 1 },
      ],
      readState: { userId: `preview-${seat}`, lastReadAt: data.lastReadAt ?? 0 },
    });
    const serialized = JSON.stringify(data.messages);
    if (lastMessages !== serialized) {
      send('chat:messages', data.messages);
      lastMessages = serialized;
    }
    document.querySelector('#status')!.textContent =
      `Local game · Revision ${revision} · BGS protocol connected`;
  } catch (error) {
    showError(String(error));
  } finally {
    fetching = false;
    if (queued) {
      queued = false;
      void refresh(true);
    }
  }
}
async function post(path: string, body: unknown) {
  const response = await fetch(path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  const data = await response.json();
  if (!response.ok) throw Error(data.error);
  return data;
}
addEventListener('message', async (event) => {
  if (
    event.source !== frame.contentWindow ||
    event.origin !== location.origin ||
    event.data?.source !== 'clash-viewer'
  )
    return;
  const { event: kind, payload } = event.data;
  if (kind === 'mounted') {
    ready = true;
    send('preferences', preferences);
    send('theme', { dark });
    await refresh(true);
  }
  if (kind === 'fetchState') await refresh(true);
  if (kind === 'update:preference') savePreference(payload.name, payload.value);
  if (kind === 'update:setting') {
    try {
      const data = await post('/api/settings', { name: payload.name, value: payload.value, seat });
      send('settings', data.settings);
    } catch (error) {
      showError(String(error));
      send('error', String(error));
    }
  }
  if (kind === 'move') {
    try {
      showError('');
      await post('/api/move', { move: payload, seat, revision });
      send('move:result', { move: payload, ok: true });
    } catch (error) {
      showError(String(error));
      send('move:result', { move: payload, ok: false, error: String(error) });
    }
    await refresh(true);
  }
  if (kind === 'replaceLog') journal = payload;
  if (kind === 'chat:read') {
    try {
      await post('/api/read', { messageId: payload.messageId, seat });
    } catch (error) {
      showError(String(error));
    }
  }
  if (kind === 'chat:send') {
    try {
      await post('/api/chat', { text: payload.text, seat });
      send('chat:result', { requestId: payload.requestId, ok: true });
      await refresh();
    } catch (error) {
      send('chat:result', { requestId: payload.requestId, ok: false, error: String(error) });
    }
  }
  if (kind === 'boardgame:clicked')
    window.open('https://boardgamers.space/boardgame/clash', '_blank', 'noopener');
  if (kind === 'player:clicked')
    document.querySelector('#status')!.textContent =
      `Player ${payload.index + 1} · ${payload.index === 0 ? 'Leif' : 'Aurelia'}`;
});
seatSelect.onchange = () => {
  lastMessages = '';
  void refresh(true);
};
document.querySelector('#reset')!.addEventListener('click', async () => {
  await post('/api/reset', {});
  lastMessages = '';
  showError('');
  await refresh(true);
});
document.querySelector('#theme')!.addEventListener('click', () => {
  dark = !dark;
  send('theme', { dark });
});
setInterval(() => {
  if (ready) void refresh();
}, 2000);
