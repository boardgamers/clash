import { viewer } from './viewer';
import { uplinkSchemas } from '@boardgamers/protocol';
const transport = viewer.launch('#app');
for (const event of Object.keys(uplinkSchemas)) {
  transport.on(event as any, (payload: unknown) =>
    parent.postMessage({ source: 'clash-viewer', event, payload }, location.origin),
  );
}
addEventListener('message', (event) => {
  if (event.source === parent && event.origin === location.origin && event.data?.source === 'clash-host')
    transport.receive(event.data.event, event.data.payload);
});
parent.postMessage({ source: 'clash-viewer', event: 'mounted' }, location.origin);
