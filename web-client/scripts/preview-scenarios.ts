import { readFileSync } from 'node:fs';

export const previewScenarios = [
  { id: 'advance', label: 'Free advance', note: 'Research lets you save an early choice.' },
  { id: 'raze', label: 'Raze city', note: 'Cities lets you save an early choice.' },
  { id: 'objectives-before', label: 'Objectives · during turn', note: 'Queue in Objectives now; conditions are checked again before scoring.' },
  { id: 'objectives', label: 'Objectives', note: 'Objectives lets you queue beside each condition.' },
  { id: 'person', label: 'Great Person', note: 'Cards lets you take or pass conditionally; earlier players have priority.' },
  { id: 'government', label: 'Keep government preference', note: 'Research has Always keep current government. Changing governments is handled on your turn.' },
  { id: 'first', label: 'Choose first player', note: 'Use Choose first player beside your civilization while Rome finishes its government choice.' },
];

export async function createPreviewScenario(engine: any, id: string) {
  if (!previewScenarios.some(s => s.id === id)) throw Error('Unknown preview scenario');
  const names: Record<string, string> = {
    advance: 'status_phase/free_advance', raze: 'status_phase/raze_city_decline',
    'objectives-before': 'objective_cards/status_phase/standing_army', objectives: 'objective_cards/status_phase/standing_army', person: 'incidents/pirates_spawn',
    government: 'status_phase/change_government', first: 'status_phase/change_government',
  };
  const game = JSON.parse(readFileSync(new URL(`../../server/tests/test_games/${names[id]}.json`, import.meta.url), 'utf8'));
  const fresh = JSON.parse(await engine.init(2, [], {}, 'early-preview', {}));
  game.players = [...game.players.slice(0, 2), ...fresh.players.slice(2)];
  if (id === 'objectives' || id === 'objectives-before') { game.players[1].objective_cards = [13]; game.players[1].resources.wood = 7; }
  if (id === 'person') {
    game.incidents_left = [55];
    game.players[1].resources = { food: 2, culture_tokens: 4 };
    game.players[1].action_cards = [7, 9, 17];
    game.action_cards_left = game.action_cards_left?.filter((card: number) => ![7, 9, 17].includes(card));
  }
  if (id === 'government' || id === 'first') { game.players[1].advances = [...game.players[0].advances]; game.players[0].resources.mood_tokens = 6; }
  let state = JSON.stringify(game);
  const apply = (action: unknown) => { state = engine.tryMove(state, JSON.stringify(action), engine.currentPlayer(state)); };
  if (id === 'advance' || id === 'objectives') apply({ Playing: 'EndTurn' });
  if (id === 'government' || id === 'first') apply({ Response: { SelectPositions: [] } });
  if (id === 'person') for (const response of [{ SelectAdvance: 'Storage' }, { SelectPositions: ['A2'] }, { SelectPositions: ['D1'] }, { Payment: [{ food: 1 }] }]) apply({ Response: response });
  const seat = id === 'objectives-before' ? 1 : 1 - engine.currentPlayer(state);
  // Check the fixture can be rendered from both seats before replacing the preview.
  for (const player of [0, 1]) engine.webView(engine.stripSecret(state, player), player);
  return { state, seat };
}
