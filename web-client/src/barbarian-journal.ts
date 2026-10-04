import type { Game, JournalEntry } from './types.ts';

/** Explain recorded event gains without inferring past eligibility from today's map. */
export function explainBarbarianGains(event: JournalEntry, game: Game): void {
  const outcomes = event.event?.outcomes ?? [];
  const base = outcomes.find((entry) =>
    entry.notes.some((note) => /^Base effect: Barbarians (move|spawn)$/.test(note)),
  );
  if (!base) return;
  const moving = base.notes.includes('Base effect: Barbarians move');
  const triggering = game.players.find((player) => player.id === base.player)?.civilization;
  const fallback = outcomes.some((entry) =>
    entry.notes.some((note) => note.startsWith('Barbarians cannot move')),
  );
  for (const entry of outcomes) {
    if (entry.civilization !== 'Barbarians') continue;
    const units = entry.tokens.filter((token) => token.icon === 'unit' && token.tone === 'gain');
    if (!units.length) continue;
    const newCity = entry.tokens.some((token) => token.icon === 'city' && token.tone === 'gain');
    let reason: string;
    if (newCity) {
      reason = fallback
        ? 'No army could move, so this event created a barbarian city with 1 infantry.'
        : 'New barbarian city · starts with 1 infantry.';
      if (!moving && units.reduce((n, token) => n + Number(token.value), 0) > 1)
        reason += ' The spawn effect also adds an extra unit to a barbarian city.';
    } else if (
      moving &&
      !fallback &&
      units.every((token) => /^(infantry|cavalry|elephant) at /.test(token.label))
    ) {
      reason = `Reinforced after barbarian movement · within 2 land spaces of ${triggering ? triggering + '’s' : 'the triggering player’s'} cities.`;
    } else if (!moving) {
      reason = 'Extra reinforcement from this event’s “Barbarians spawn” effect.';
    } else continue;
    entry.notes.push(reason);
    entry.text += ' · ' + reason;
  }
}
