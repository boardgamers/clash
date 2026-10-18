import type { BoardFrame, Game, View, Pile } from './types.ts';
import { frameResources, describeResources } from './resource-playback.ts';
import { frameHasContent } from './replay-actions.ts';
import { influenceRecap } from './influence-recap.ts';
import { activeHistory } from './active-history.ts';
import { officialWonderText, wonderName } from './wonder-names.ts';
import { printedCardName, printedCardText } from './card-names.ts';

export function recapStart(game: Game, seat: number | undefined, seen = 0): number | null {
  if (game.state === 'Finished') return null;
  const frames = game.board_history?.frames ?? [];
  if (seat === undefined || frames.length < 2 || seen >= frames.at(-1)!.cursor) return null;
  const responses = latestTurnResponses(game, frames, seat);
  if (responses && !frames.slice(responses.end + 1).some((f) => f.actor === seat)) {
    const start = frameAt(frames, Math.max(seen, frames[responses.start].cursor));
    return start < responses.end ? start : null;
  }
  const lastTurn = lastIndex(frames, (f) => f.actor === seat && f.ended_turn);
  if (lastTurn < 0 || frames.slice(lastTurn + 1).some((f) => f.actor === seat)) return null;
  const start = lastIndex(frames, (f) => f.cursor <= Math.max(seen, frames[lastTurn].cursor));
  return start >= 0 && start < frames.length - 1 && hasReplayContent(game, frames, start, frames.length - 1)
    ? start
    : null;
}
export function frameAt(frames: BoardFrame[], cursor: number): number {
  return Math.max(
    0,
    lastIndex(frames, (frame) => frame.cursor <= cursor),
  );
}

/** Replay all intervening turns, including the position before their first action. */
export function sinceLastTurn(game: Game, seat: number | undefined) {
  const frames = game.board_history?.frames ?? [];
  if (seat === undefined) return null;
  const responses = latestTurnResponses(game, frames, seat);
  if (responses) return responses;
  let end = frames.length - 1;
  while (end > 0) {
    const boundary = lastIndex(frames.slice(0, end), (f) => f.ended_turn);
    const owner = frames[end].ended_turn
      ? frames[end].actor
      : frames.slice(boundary + 1, end + 1).find((f) => f.actor !== null)?.actor;
    const start = Math.max(0, boundary);
    if (owner != null && owner !== seat && start < end) {
      const ownEnd = lastIndex(frames.slice(0, end), (f) => f.actor === seat && f.ended_turn);
      const first = Math.max(0, ownEnd);
      return hasReplayContent(game, frames, first, end) ? { start: first, end } : null;
    }
    end = boundary;
  }
  return null;
}

/** Other players can act during your turn, for example after you capture a city. */
function latestTurnResponses(game: Game, frames: BoardFrame[], seat: number) {
  const end = frames.length - 1;
  if (end < 1) return null;
  const boundary = lastIndex(frames.slice(0, end), (f) => f.ended_turn);
  const owner = frames[end].ended_turn
    ? frames[end].actor
    : frames.slice(boundary + 1).find((f) => f.actor !== null)?.actor;
  if (owner !== seat) return null;
  const response = lastIndex(
    frames,
    (f) => f.actor !== null && f.actor !== seat && frameHasContent(game, frames[frames.indexOf(f) - 1], f),
  );
  if (response <= boundary) return null;
  const start = lastIndex(frames.slice(0, response), (f) => f.actor === seat);
  return start > boundary ? { start, end: response } : null;
}

/** Describe public board changes and the journal entries belonging to each frame. */
export function frameDetails(
  before: BoardFrame | undefined,
  frame: BoardFrame | null,
  game?: Game | null,
  advances: View['advances'] = [],
  cards: NonNullable<View['cardCatalog']> = [],
) {
  if (!frame) return { caption: 'No recorded positions yet.', positions: [] as string[] };
  let actor = frame.players.find((p) => p.id === frame.actor)?.civilization;
  const positions = new Set<string>();
  const units = (f: BoardFrame | undefined) =>
    new Map(
      (f?.players ?? []).flatMap((p) =>
        (p.units ?? []).flatMap((u) =>
          [u, ...(u.carried_units ?? []).map((c) => ({ ...c, position: u.position }))].map(
            (unit) => [`${p.id}:${unit.id}`, unit] as const,
          ),
        ),
      ),
    );
  const oldUnits = units(before),
    newUnits = units(frame);
  if (before) {
    for (const [id, unit] of newUnits) {
      const old = oldUnits.get(id);
      if (!old || old.position !== unit.position) {
        positions.add(unit.position);
        if (old) positions.add(old.position);
      }
    }
    for (const [id, unit] of oldUnits) if (!newUnits.has(id)) positions.add(unit.position);
    const cities = (f: BoardFrame) =>
      new Map(
        f.players.flatMap((p) =>
          (p.cities ?? []).map((city) => [city.position, { owner: p.id, city }] as const),
        ),
      );
    const oldCities = cities(before),
      newCities = cities(frame);
    for (const position of new Set([...oldCities.keys(), ...newCities.keys()]))
      if (JSON.stringify(oldCities.get(position)) !== JSON.stringify(newCities.get(position)))
        positions.add(position);
    const tiles = new Map(before.tiles);
    for (const [position, terrain] of frame.tiles)
      if (JSON.stringify(tiles.get(position)) !== JSON.stringify(terrain)) positions.add(position);
  }
  let description = ['Explore Resolution', 'Finish ship exploration'].includes(frame.title)
    ? 'Explore'
    : frame.title;
  let generic = [
    'Resolve choice',
    'Pay for action',
    'Action',
    'Select Objective Cards to Complete',
    'Complete Objectives',
  ].includes(frame.title);
  const outcomes: string[] = [];
  if (before && generic) {
    for (const player of frame.players) {
      for (const [from, to, verb] of [
        [oldUnits, newUnits, 'placed'],
        [newUnits, oldUnits, 'lost'],
      ] as const) {
        const counts = new Map<string, number>();
        for (const [id, unit] of to) {
          if (from.has(id) || !id.startsWith(`${player.id}:`)) continue;
          const name = typeof unit.unit_type === 'string' ? unit.unit_type.toLowerCase() : 'leader';
          const key = `${name} at ${unit.position}`;
          counts.set(key, (counts.get(key) ?? 0) + 1);
        }
        for (const [unit, count] of counts)
          outcomes.push(
            `${player.id === frame.actor ? '' : player.civilization + ': '}${verb} ${count} ${count > 1 ? unit.replace(/^(ship|settler|elephant|leader) at /, '$1s at ') : unit}`,
          );
      }
    }
  }
  if (before && frame.title === 'Recruit') {
    const counts = new Map<string, number>();
    for (const [id, unit] of newUnits) {
      if (oldUnits.has(id) || !id.startsWith(`${frame.actor}:`)) continue;
      const name = typeof unit.unit_type === 'string' ? unit.unit_type.toLowerCase() : 'leader';
      counts.set(name, (counts.get(name) ?? 0) + 1);
    }
    if (counts.size)
      description =
        'recruited ' +
        [...counts]
          .map(
            ([name, count]) =>
              `${count} ${name}${count > 1 && !['infantry', 'cavalry'].includes(name) ? 's' : ''}`,
          )
          .join(', ');
  }
  // Read only the public log interval for this frame, never the current player's hand
  // or later turns. This also enriches recordings made before detailed captions existed.
  if (before && game) {
    const history = activeHistory(game)
      .flatMap((age) => age.rounds.flatMap((round) => round.turns.flatMap((turn) => turn.actions ?? [])))
      .slice(0, frame.cursor);
    const actions = history.slice(before.cursor);
    const influence = influenceRecap(history, before.cursor, before, frame);
    if (influence) {
      description = influence.description;
      actor = influence.actor ?? actor;
      generic = false;
      influence.positions.forEach((position) => positions.add(position));
    }
    const items = actions.flatMap((action) => action.items ?? []);
    if (frame.title === 'Play action card' && !influence) {
      const played = actions.flatMap((action) => {
        const playing = typeof action.action === 'object' && action.action?.Playing;
        return playing &&
          typeof playing === 'object' &&
          'ActionCard' in playing &&
          typeof playing.ActionCard === 'number'
          ? [playing.ActionCard]
          : [];
      });
      const names = [
        ...new Set(played.map((id) => cards.find((card) => card.id === id)?.name).filter(Boolean)),
      ];
      if (names.length) description = `Played ${names.join(' · ')}`;
    }
    if (frame.title === 'Civilization ability' && !influence) {
      // Older frames kept a generic title. The public start entry already names
      // the advance or leader ability, including free actions with no cost item.
      for (const action of actions) {
        const playing = typeof action.action === 'object' && action.action?.Playing;
        if (!playing || typeof playing !== 'object' || !('Custom' in playing)) continue;
        const special = (playing.Custom as { action?: string })?.action;
        const structuredName =
          special &&
          action.items?.find((item) =>
            Object.values(item.origin ?? {}).some(
              (name) => typeof name === 'string' && name.replaceAll(' ', '') === special,
            ),
          )?.origin;
        const name =
          (structuredName &&
            Object.values(structuredName)
              .find((name): name is string => typeof name === 'string')
              ?.replace(/([a-z])([A-Z])/g, '$1 $2')) ??
          action.log
            ?.map((line) => line.match(/: ([^:]+): (?:.*?, )?Start action(?: in city [A-Z]\d+)?(?:,|$)/)?.[1])
            .find(Boolean);
        if (name) {
          description = name;
          break;
        }
      }
    }
    const details: string[] = influence ? [] : [...outcomes];
    const readable = wonderName;
    for (const player of frame.players) {
      const own = items.filter((item) => item.player === player.id);
      const completed = [
        ...new Set(
          own.flatMap((item) => {
            const to = item.HandCard?.to;
            return to &&
              typeof to === 'object' &&
              'CompleteObjective' in to &&
              typeof to.CompleteObjective === 'string'
              ? [to.CompleteObjective]
              : [];
          }),
        ),
      ];
      if (completed.length)
        details.push(
          `${player.id === frame.actor ? '' : player.civilization + ': '}Completed ${completed.join(', ')}`,
        );
      const gained = [
        ...new Set(
          own.filter((item) => item.Advance?.balance === 'Gain').map((item) => item.Advance!.advance),
        ),
      ];
      if (gained.length)
        details.push(
          `${player.id === frame.actor ? '' : player.civilization + ': '}researched ${gained
            .map((id) => advances.find((a) => a.id === id)?.name ?? readable(id))
            .join(', ')}`,
        );
      if (player.id !== frame.actor) continue;
      if (generic || frame.title === 'Build' || frame.title === 'Build wonder') {
        const built = own.flatMap((item) => {
          const structure = item.Structure;
          if (
            structure?.balance !== 'Gain' ||
            !structure.structure ||
            typeof structure.structure !== 'object'
          )
            return [];
          const value = Object.values(structure.structure)[0];
          return typeof value === 'string' ? [readable(value)] : [];
        });
        if (built.length) details.push((generic ? 'gained ' : 'built ') + [...new Set(built)].join(', '));
      }
    }
    if (generic) {
      for (const player of frame.players) {
        const prefix = player.id === frame.actor ? '' : player.civilization + ': ';
        const paid: Pile = {};
        for (const item of items.filter((item) => item.player === player.id)) {
          if (item.Resources?.balance === 'Pay')
            for (const [resource, amount] of Object.entries(item.Resources.resources))
              paid[resource as keyof typeof paid] =
                (paid[resource as keyof typeof paid] ?? 0) + (amount ?? 0);
          if (item.MoodChange)
            details.push(`${prefix}${item.MoodChange.city} is now ${item.MoodChange.mood.toLowerCase()}`);
        }
        const payment = describeResources(paid);
        if (payment) details.push(`${prefix}paid ${payment}`);
      }
      // Public effect labels identify card types without exposing private cards.
      for (const effect of frame.effects ?? []) {
        if (
          effect.kind === 'completed' &&
          items.some((item) => {
            const to = item.HandCard?.to;
            return (
              item.player === effect.player &&
              to &&
              typeof to === 'object' &&
              'CompleteObjective' in to &&
              to.CompleteObjective === effect.label
            );
          })
        )
          continue;
        const player = frame.players.find((p) => p.id === effect.player);
        const label = `${effect.player === frame.actor ? '' : (player?.civilization ?? 'Player') + ': '}${effect.label}`;
        if (!details.includes(label)) details.push(label);
      }
    }
    const resources = frameResources(game, before.cursor, frame);
    for (const marker of resources.markers) if (marker.position) positions.add(marker.position);
    for (const gain of resources.gains) {
      const player = frame.players.find((p) => p.id === gain.player)?.civilization;
      const gained = describeResources(gain.pile),
        waste = describeResources(gain.waste);
      const prefix = gain.player === frame.actor ? '' : `${player ?? 'Player'}: `;
      if (gained) details.push(`${prefix}${frame.title === 'Collect' ? 'collected' : 'gained'} ${gained}`);
      if (waste) details.push(`${prefix}${waste} not stored`);
    }
    if (generic && !details.length) {
      const result = items
        .map((item) => item.Text)
        .find(
          (text) =>
            text &&
            /^(Decided to |Declined |Removed |Completed |Selected |Chose |Kept |Discarded |Passed)/i.test(
              text,
            ),
        );
      if (result) details.push(result);
    }
    if (details.length)
      description =
        (generic && description === frame.title) ||
        ['Research', 'Build', 'Build wonder', 'Collect'].includes(frame.title)
          ? details.join(' · ')
          : [description, ...details].join(' · ');
  }
  if (generic && description === frame.title) {
    const action =
      game &&
      activeHistory(game)
        .flatMap((a) => a.rounds.flatMap((r) => r.turns.flatMap((t) => t.actions ?? [])))
        .slice(before?.cursor ?? frame.cursor, frame.cursor)
        .at(-1)?.action;
    const response = action && typeof action === 'object' ? action.Response : null;
    const kind = response && typeof response === 'object' ? Object.keys(response)[0] : '';
    description =
      (
        {
          SelectAdvance: 'Research',
          SelectUnits: 'Selected units',
          SelectPositions: 'Selected a location',
          Payment: 'Payment',
          ResourceReward: 'Resource reward',
          ExploreResolution: 'Explore',
          Bool: 'Event decision',
        } as Record<string, string>
      )[kind] ?? 'Turn update';
  }
  return {
    caption: officialWonderText(
      actor
        ? `${actor} · ${printedCardText(printedCardName(description))}`
        : printedCardText(printedCardName(description)),
    ),
    positions: [...positions],
  };
}
export function frameEffects(game: Game, after: number) {
  return (game.board_history?.frames ?? [])
    .filter((f) => f.cursor > after)
    .flatMap((f) =>
      (f.effects ?? [])
        .filter((effect) => (effect.cursor ?? f.cursor) > after)
        .map((effect, i) => ({
          ...effect,
          key: effect.key ?? `${game.board_history!.id}:${f.cursor}:${i}`,
        })),
    );
}

function lastIndex<T>(items: T[], match: (item: T) => boolean) {
  for (let i = items.length - 1; i >= 0; i--) if (match(items[i])) return i;
  return -1;
}

function hasReplayContent(game: Game, frames: BoardFrame[], start: number, end: number) {
  return frames.some(
    (frame, index) => index > start && index <= end && frameHasContent(game, frames[index - 1], frame),
  );
}
