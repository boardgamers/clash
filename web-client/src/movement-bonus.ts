import type { Game } from './types.ts';

export interface MovementBonus {
  source: string;
  label: string;
  key: string;
}

// Movement state is authoritative; history identifies who granted that movement.
// Undone actions remain in older saves, but their logs and items are cleared.
export function movementBonus(game: Game | null): MovementBonus | null {
  if (!game?.state || typeof game.state !== 'object' || !('Movement' in game.state)) return null;
  const age = game.log?.at(-1);
  const round = age?.rounds.at(-1);
  const turn = round?.turns.at(-1);
  const actions = turn?.actions ?? [];
  for (let i = actions.length - 1; i >= 0; i--) {
    const entry = actions[i];
    if (!entry.log?.length && !entry.items?.length) continue;
    const action = entry.action;
    if (!action || typeof action !== 'object') continue;
    if ('Movement' in action) {
      // A paid move starts a fresh allowance, even after Expansion in the same turn.
      if (action.Movement === 'Stop' || entry.items?.some((item) => item.Action?.balance === 'Loss'))
        return null;
      continue;
    }
    if (!('Playing' in action)) continue;
    const playing = action.Playing;
    if (!playing || typeof playing !== 'object') return null;
    const key = `${age?.age}/${round?.round}/${round?.turns.length}/${i}`;
    if ('Recruit' in playing && entry.log?.some((line) => line.includes(': Expansion: Expansion allows')))
      return { source: 'Expansion', label: 'Free settler moves', key };
    if ('ActionCard' in playing && playing.ActionCard === 24)
      return { source: 'Great Warlord', label: 'Bonus Move action', key };
    return null;
  }
  return null;
}
