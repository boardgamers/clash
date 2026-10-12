import { test } from 'node:test';
import assert from 'node:assert/strict';
import { boardContentSignature } from './board-content.ts';
import type { Session } from './types.ts';

function session(): Session {
  return {
    game: { players: [{ id: 0, civilization: 'India', resources: { food: 2 },
      cities: [{ position: 'A1', mood_state: 'Happy', city_pieces: { temple: 0 }, activations: 0 }],
      units: [{ id: 0, position: 'A1', unit_type: 'Infantry' }],
    }] },
    view: { players: [{ index: 0, capital: 'A1', resources: { food: 2 } }] },
    seat: 0, colorBlind: false, playerColors: [], playerSymbols: [], strategyMap: false,
  } as unknown as Session;
}
const signature = (s: Session) => boardContentSignature(s, [['A1', 'Fertile']]);

test('resource, research, activation and control changes retain the board', () => {
  const before = session(), after = structuredClone(before);
  after.game!.players[0].resources = { food: 7 };
  after.game!.players[0].advances = ['Theaters'];
  after.game!.players[0].cities![0].activations = 1;
  Object.assign(after.game!.players[0].units![0], { movement_restrictions: ['Battle'] });
  Object.assign(after.view!.players[0], { resources: { food: 7 }, score: 20 });
  after.pending = true;
  after.activityOpen = true;
  assert.equal(signature(after), signature(before));
});

test('visible map changes invalidate models in both map modes', () => {
  const changes: ((s: Session) => void)[] = [
    s => { s.game!.players[0].cities![0].mood_state = 'Angry'; },
    s => { s.game!.players[0].cities![0].city_pieces!.academy = 0; },
    s => { s.game!.players[0].cities![0].city_pieces!.temple = 1; },
    s => { s.game!.players[0].cities![0].port_position = 'B1'; },
    s => { s.game!.players[0].units![0].position = 'B1'; },
    s => { s.game!.players[0].units![0].unit_type = { Leader: 'Ashoka' }; },
    s => { s.game!.players[0].units![0].carried_units = [{ id: 1, unit_type: 'Infantry' }]; },
    s => { s.game!.players[0].units![0].carrier_id = 2; },
    s => { s.game!.players[0].civilization = 'Rome'; },
    s => { s.view!.players[0].capital = 'B1'; },
    s => { s.playerColors = ['#ffffff']; },
    s => { s.colorBlind = true; },
  ];
  for (const strategyMap of [false, true]) {
    const before = session(); before.strategyMap = strategyMap;
    for (const change of changes) {
      const after = structuredClone(before); change(after);
      assert.notEqual(signature(after), signature(before));
    }
    assert.notEqual(boardContentSignature(before, [['A1', 'Mountain']]), signature(before));
  }
});
