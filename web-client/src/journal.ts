import { readableHistory } from './structured-log.ts';
import { activeHistory } from './active-history.ts';
import type { Game, JournalEntry, JournalToken, LoggedAction, Pile, Player, Resource } from './types.ts';
import type { View } from './types.ts';
import { explainEvents } from './event-journal.ts';
import { collectionCities, explainCollection } from './collection-journal.ts';

const resourceIcons: Record<string, Resource | 'action'> = {
  food: 'food',
  wood: 'wood',
  ore: 'ore',
  idea: 'ideas',
  ideas: 'ideas',
  gold: 'gold',
  'mood token': 'mood_tokens',
  'mood tokens': 'mood_tokens',
  'culture token': 'culture_tokens',
  'culture tokens': 'culture_tokens',
  action: 'action',
  actions: 'action',
};

function kind(action: LoggedAction): JournalEntry['kind'] {
  if (action.combat_stats) return 'combat';
  if (
    action.items?.some(
      (item) =>
        item.HandCard?.to && typeof item.HandCard.to === 'object' && 'CompleteObjective' in item.HandCard.to,
    )
  )
    return 'objective';
  if (!action.action || typeof action.action === 'string') return 'event';
  if ('Movement' in action.action) return 'move';
  const playing = action.action.Playing;
  const name =
    typeof playing === 'string'
      ? playing
      : playing && typeof playing === 'object'
        ? Object.keys(playing)[0]
        : '';
  const kinds: Record<string, JournalEntry['kind']> = {
    Collect: 'collect',
    Advance: 'research',
    EndTurn: 'end-turn',
    Construct: 'build',
    FoundCity: 'build',
    Recruit: 'recruit',
    ActionCard: 'card',
    WonderCard: 'card',
  };
  return kinds[name] ?? 'event';
}

function factionPlayer(game: Game, name: string): Player | undefined {
  return game.players.find(
    (p) =>
      p.name === name ||
      p.civilization === name ||
      name === `Player${p.id + 1}` ||
      name === `Player ${p.id + 1}`,
  );
}

function factionText(game: Game, text: string): string {
  // Replace actors and explicit player references, never resource or objective names.
  const names = game.players.flatMap((p) =>
    [p.name, `Player${p.id + 1}`, `Player ${p.id + 1}`]
      .filter((n): n is string => !!n)
      .map((name) => ({ name, civilization: p.civilization })),
  );
  const aliases = new Map(names.map((n) => [n.name, n.civilization]));
  const escaped = [...aliases.keys()]
    .sort((a, b) => b.length - a.length)
    .map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  if (!escaped.length) return text;
  return text.replace(
    new RegExp(`(^|\\b(?:from|to|by|against|with|of) )(${escaped.join('|')})(?![\\p{L}\\p{N}_])`, 'gu'),
    (_match, prefix: string, name: string) => prefix + aliases.get(name)!,
  );
}

function delta(value: number, label: string, icon: JournalToken['icon'], compact = false): JournalToken {
  return {
    icon,
    value: `${value === 0 ? '' : value < 0 ? '−' : '+'}${Math.abs(value)}`,
    label,
    compact,
    description: `${value === 0 ? '' : value < 0 ? '−' : '+'}${Math.abs(value)} ${label}`,
    tone: value === 0 ? undefined : value < 0 ? 'loss' : 'gain',
  };
}

type PaymentLog = { entry: JournalEntry; tokens: JournalToken[]; pile: Pile; note?: string };

function formatCombatRolls(text: string): string {
  return text.replace(
    /\bRoll (.+?) for combined combat value of (\d+) and gets (\d+) hits against (attacking|defending) units/g,
    (_match, rolls: string, value: string, hits: string, opponent: string) => {
      // A face is part of the roll even when its ability doesn't activate.
      // Name it explicitly so naval faces aren't mistaken for fighting units.
      const dice = rolls.replace(/\((infantry|cavalry|elephant|leader), /g, '($1 face: ');
      return `Roll ${dice} → combat value ${value} → ${hits} ${hits === '1' ? 'hit' : 'hits'} against ${opponent} units`;
    },
  );
}

// Playing actions can announce a cost before opening a payment request. Only
// resource-loss items record money actually spent. Match backwards because the
// confirmed payment follows the announcement when both resolve in one action.
function confirmPayments(action: LoggedAction, payments: PaymentLog[], entries: JournalEntry[]): void {
  const playing = typeof action.action === 'object' && action.action?.Playing;
  const deferred =
    playing &&
    typeof playing === 'object' &&
    ('ActionCard' in playing ||
      'Custom' in playing ||
      Object.values(playing).some(
        (v) => v && typeof v === 'object' && 'action_type' in v && typeof v.action_type === 'object',
      ));
  if (!deferred) return;
  const balances = new Map<number, Pile>();
  for (const item of action.items ?? []) {
    if (!item.Resources || !['Loss', 'Pay'].includes(item.Resources.balance)) continue;
    const balance = balances.get(item.player) ?? {};
    for (const [resource, amount] of Object.entries(item.Resources.resources))
      balance[resource as Resource] = (balance[resource as Resource] ?? 0) + amount;
    balances.set(item.player, balance);
  }
  for (const payment of [...payments].reverse()) {
    if (payment.entry.player === undefined) continue;
    const balance = balances.get(payment.entry.player) ?? {};
    const amounts = Object.entries(payment.pile) as [Resource, number][];
    if (amounts.every(([resource, amount]) => (balance[resource] ?? 0) >= amount)) {
      for (const [resource, amount] of amounts) balance[resource] = (balance[resource] ?? 0) - amount;
    } else {
      payment.entry.tokens = payment.entry.tokens.filter((token) => !payment.tokens.includes(token));
      if (payment.note) payment.entry.notes = payment.entry.notes.filter((note) => note !== payment.note);
      if (!payment.entry.tokens.length && !payment.entry.notes.length)
        entries.splice(entries.indexOf(payment.entry), 1);
    }
  }
}

function addClause(entry: JournalEntry, clause: string, payments: PaymentLog[]): void {
  let match = clause.match(/^(Pay|Gain|Added|Lose|Lost) (.+?)(?: with (.+))?$/);
  if (match) {
    const sign = match[1] === 'Gain' || match[1] === 'Added' ? 1 : -1;
    const quantities = match[2]
      .split(/,\s*(?:and\s+)?|\s+and\s+|\s*\+\s*/)
      .map((part) =>
        part.match(/^(\d+(?:\.\d+)?) (food|wood|ore|ideas?|gold|mood tokens?|culture tokens?|actions?)$/),
      );
    if (quantities.every((q) => q)) {
      const tokens = quantities.map((q) => delta(sign * Number(q![1]), q![2], resourceIcons[q![2]], true));
      entry.tokens.push(...tokens);
      const note = match[3] ? `With ${match[3]}` : undefined;
      if (note) entry.notes.push(note);
      if (match[1] === 'Pay' && tokens.every((token) => token.icon !== 'action')) {
        const pile: Pile = {};
        for (const q of quantities) {
          const resource = resourceIcons[q![2]] as Resource;
          pile[resource] = (pile[resource] ?? 0) + Number(q![1]);
        }
        payments.push({ entry, tokens, pile, note });
      }
      return;
    }
  }
  match = clause.match(/^Convert (\d+) (food|wood|ore|ideas?|gold) to (\d+) (food|wood|ore|ideas?|gold)$/);
  if (match) {
    entry.tokens.push(
      delta(-Number(match[1]), match[2], resourceIcons[match[2]], true),
      delta(Number(match[3]), match[4], resourceIcons[match[4]], true),
    );
    return;
  }
  match = clause.match(/^Use city ([A-Z]+\d+)$/);
  if (match) {
    entry.title += ` · ${match[1]}`;
    return;
  }
  match = clause.match(/^(Draw|Discard) (?:an? |1 )(action|objective|wonder) card$/);
  if (match) {
    const type = match[2];
    entry.tokens.push(
      delta(
        match[1] === 'Draw' ? 1 : -1,
        `${type[0].toUpperCase()}${type.slice(1)} card`,
        type === 'action' ? 'card' : (type as 'objective' | 'wonder'),
      ),
    );
    return;
  }
  match = clause.match(/^Gain (.+?) (?:and take an event token \((.+)\)|without taking an event token)$/);
  if (match) {
    entry.tokens.push(delta(1, match[1], 'research'));
    entry.tokens.push({
      icon: 'event',
      label: match[2]
        ? match[2] === 'triggering an incident'
          ? 'Incident triggered'
          : `Event tokens: ${match[2]}`
        : 'No event token',
      description: match[2] ? `Take an event token · ${match[2]}` : 'Advance without taking an event token',
    });
    return;
  }
  match = clause.match(/^(Gain|Lose) city ([A-Z]+\d+)$/);
  if (match) {
    entry.tokens.push(delta(match[1] === 'Gain' ? 1 : -1, `City ${match[2]}`, 'city'));
    return;
  }
  match = clause.match(
    /^(Gain|Lose|Lost) (\d+) (settlers?|infantry|cavalry|elephants?|ships?)( at [A-Z]+\d+)?$/,
  );
  if (match) {
    entry.tokens.push(
      delta((match[1] === 'Gain' ? 1 : -1) * Number(match[2]), match[3] + (match[4] ?? ''), 'unit'),
    );
    return;
  }
  match = clause.match(/^City ([A-Z]+\d+) became (Happy|Neutral|Angry)$/);
  if (match) {
    entry.tokens.push({
      icon: match[2].toLowerCase() as 'happy' | 'neutral' | 'angry',
      label: `${match[1]} ${match[2]}`,
      description: `City ${match[1]} became ${match[2]}`,
    });
    return;
  }
  if (entry.kind === 'end-turn' && clause === '0 actions left') return;
  // Unrecognized rules, combat details, and named/private card text remain intact.
  entry.notes.push(clause);
}

export function journal(
  game: Game,
  view?: Pick<View, 'eventCatalog' | 'pendingEvent' | 'players' | 'logOriginNames'>,
): JournalEntry[] {
  game = readableHistory(game, view);
  const historicalCities = collectionCities(game);
  const eventNames = [
    ...new Set([
      ...(view?.eventCatalog?.map((e) => e.name) ?? []),
      ...activeHistory(game).flatMap((a) =>
        a.rounds.flatMap((r) =>
          r.turns.flatMap((t) =>
            (t.actions ?? []).flatMap((action) =>
              (action.log ?? []).flatMap(
                (line) => line.match(/^A new game event has been triggered: (.+)$/)?.[1] ?? [],
              ),
            ),
          ),
        ),
      ),
    ]),
  ].sort((a, b) => b.length - a.length);
  const entries = activeHistory(game).flatMap((age, a) =>
    age.rounds.flatMap((round, r) =>
      round.turns.flatMap((turn, t) =>
        (turn.actions ?? []).flatMap((action, c) => {
          const base = { age: age.age, round: round.round };
          const id = `${a}-${r}-${t}-${c}`;
          if (turn.turn_type === 'Setup' || action.action === 'Setup') {
            const players = [...new Set((action.items ?? []).map((item) => item.player))];
            const setups = players.length
              ? players.flatMap((id) => {
                  const player = game.players.find((p) => p.id === id);
                  if (!player) return [];
                  const position = action.items?.find(
                    (item) =>
                      item.player === id &&
                      item.Structure?.structure === 'CityCenter' &&
                      item.Structure.balance === 'Gain',
                  )?.Structure?.position;
                  return [{ player: id, civilization: player.civilization, position }];
                })
              : (action.log ?? []).flatMap((text) => {
                  const match = text.match(/^(.+?): Setup: Play as ([^,]+)/);
                  return match
                    ? [
                        {
                          player: factionPlayer(game, match[1])?.id,
                          civilization: match[2],
                          position: text.match(/\bGain city ([A-Z]+\d+)\b/)?.[1],
                        },
                      ]
                    : [];
                });
            return setups.map((setup, i): JournalEntry => ({
              ...base,
              id: `${id}-${i}`,
              player: setup.player,
              civilization: setup.civilization,
              setup,
              kind: 'setup',
              title: `Start${setup.position ? ` · ${setup.position}` : ''}`,
              tokens: [],
              notes: [],
              text: `${setup.civilization}${setup.position ? ` · starts at ${setup.position}` : ''}`,
            }));
          }
          const entries: JournalEntry[] = [];
          const payments: PaymentLog[] = [];
          for (const rawLine of action.log ?? []) {
            const line = rawLine.includes(': Combat: ') ? formatCombatRolls(rawLine) : rawLine;
            const eventName = line.match(/^A new game event has been triggered: (.+)$/)?.[1];
            if (eventName) {
              entries.push({
                ...base,
                id: `${id}-${entries.length}`,
                kind: 'event',
                title: eventName,
                text: '',
                tokens: [],
                notes: [],
                event: {
                  outcomes: [],
                  explanations: [],
                  triggeredBy: action.player ?? entries.at(-1)?.player,
                },
              });
              continue;
            }
            const actorLine = line.match(/^([^:]+): (.*)$/s);
            const eventSource = actorLine && eventNames.find((name) => actorLine[2].startsWith(`${name}: `));
            const match =
              eventSource && actorLine
                ? [line, actorLine[1], eventSource, actorLine[2].slice(eventSource.length + 2)]
                : line.match(/^([^:]+): ([^:]+): (.*)$/s);
            const origin = match?.[2];
            // Origin metadata also identifies actors in old logs after a display-name change.
            const originPlayers = [
              ...new Set(
                action.items
                  ?.filter(
                    (item) =>
                      Object.values(item.origin ?? {}).includes(origin) ||
                      view?.eventCatalog?.some(
                        (event) => event.name === origin && event.id === item.origin?.Incident,
                      ),
                  )
                  .map((item) => item.player),
              ),
            ];
            const player = match
              ? (factionPlayer(game, match[1]) ??
                (originPlayers.length === 1
                  ? game.players.find((p) => p.id === originPlayers[0])
                  : undefined))
              : undefined;
            const civilization = player?.civilization;
            // Only combine adjacent rows from the same action and actor; keep chronological order.
            let entry = entries.at(-1);
            const sameSource =
              entry &&
              (entry.title.split(' · ')[0].toLowerCase() === origin?.toLowerCase() ||
                (entry.title === 'Research' &&
                  (origin === 'Advance' ||
                    entry.tokens.some((token) => token.icon === 'research' && token.label === origin))));
            if (!player || !entry || entry.player !== player.id || !sameSource) {
              const title =
                origin === 'Advance'
                  ? 'Research'
                  : origin === 'End Turn'
                    ? 'End turn'
                    : origin === 'Found City'
                      ? 'Found city'
                      : (origin ?? '');
              entry = {
                ...base,
                id: `${id}-${entries.length}`,
                kind: kind(action),
                player: player?.id,
                civilization,
                title,
                tokens: [],
                notes: origin === 'Place Settler' ? ['Free settler after losing a city.'] : [],
                text: '',
              };
              entries.push(entry);
            }
            if (!player || !match) {
              entry.notes.push(factionText(game, line));
              continue;
            }
            for (const clause of match[3].split(/,(?!\s*(?:\d|and\b))\s*/))
              addClause(entry, factionText(game, clause), payments);
          }
          confirmPayments(action, payments, entries);
          explainCollection(action, entries, historicalCities.get(id));
          for (const entry of entries)
            entry.text = [
              entry.civilization,
              entry.title,
              ...entry.tokens.map((token) => token.description),
              ...entry.notes,
              ...(entry.collection?.effects.map(
                (effect) =>
                  `${effect.source} (included): ${effect.tokens.map((t) => t.description).join(', ')}`,
              ) ?? []),
            ]
              .filter(Boolean)
              .join(' · ');
          return entries;
        }),
      ),
    ),
  );
  return explainEvents(entries, game, view);
}
