import { activeHistory } from './active-history.ts';
import { wonderName } from './wonder-names.ts';
import { explainBarbarianGains } from './barbarian-journal.ts';
import type { EventInfo, Game, JournalEntry, LoggedAction, View } from './types.ts';

type Facts = { units: number; cities: number; advances: Set<string>; borrowed?: string };
type RecordAtAction = { action: LoggedAction; facts: Map<number, Facts>; actor?: number };
const actionId = (entry: JournalEntry) => entry.id.split('-').slice(0, 4).join('-');
const displayName = wonderName;

function publicHistory(game: Game): Map<string, RecordAtAction> {
  const facts = new Map(
    game.players.map((p) => [
      p.id,
      {
        units: (p.units ?? []).reduce((n, u) => n + 1 + (u.carried_units?.length ?? 0), 0),
        cities: p.cities?.length ?? 0,
        advances: new Set(p.advances ?? []),
      },
    ]),
  );
  const actions = activeHistory(game).flatMap((age, a) =>
    age.rounds.flatMap((round, r) =>
      round.turns.flatMap((turn, t) =>
        (turn.actions ?? []).map((action, c) => ({
          id: `${a}-${r}-${t}-${c}`,
          action,
          actor:
            typeof turn.turn_type === 'object' && turn.turn_type !== null && 'Player' in turn.turn_type
              ? (turn.turn_type as { Player: number }).Player
              : undefined,
          borrowed: (turn.actions ?? [])
            .slice(0, c + 1)
            .flatMap((a) => a.log ?? [])
            .map((line) => line.match(/: Great Library: Use (.+) for the turn$/)?.[1])
            .filter(Boolean)
            .at(-1),
        })),
      ),
    ),
  );
  const result = new Map<string, RecordAtAction>();
  for (const record of actions.reverse()) {
    // Undone actions have their outcomes cleared; never replay their command as an effect.
    if (!record.action.log?.length && !record.action.items?.length) continue;
    result.set(record.id, {
      ...record,
      facts: record.action.log?.some((line) => line.startsWith('A new game event has been triggered: '))
        ? new Map([...facts].map(([id, f]) => [id, { ...f, advances: new Set(f.advances) }]))
        : new Map(),
    });
    const actorFacts =
      record.actor !== undefined ? result.get(record.id)?.facts.get(record.actor) : undefined;
    if (actorFacts) actorFacts.borrowed = record.borrowed;
    for (const item of [...(record.action.items ?? [])].reverse()) {
      const p = facts.get(item.player);
      if (!p) continue;
      if (item.Units)
        p.units -=
          (item.Units.balance === 'Gain' ? 1 : -1) *
          Object.entries(item.Units.units).reduce(
            (n, [key, value]) => n + (key === 'leader' ? 1 : typeof value === 'number' ? value : 0),
            0,
          );
      if (item.Structure?.structure === 'CityCenter') p.cities -= item.Structure.balance === 'Gain' ? 1 : -1;
      if (item.Advance) {
        if (item.Advance.balance === 'Gain') p.advances.delete(item.Advance.advance);
        else p.advances.add(item.Advance.advance);
      }
    }
  }
  return result;
}

function eventInfo(name: string, records: RecordAtAction[], catalog: EventInfo[]): EventInfo | undefined {
  const candidates = catalog.filter((event) => event.name === name);
  const ids = records.flatMap((record) => record.action.items?.map((item) => item.origin?.Incident) ?? []);
  const exact = candidates.filter((event) => ids.includes(event.id));
  if (exact.length === 1) return exact[0];
  // Different cards can share a title. Only fall back when their rules also agree.
  if (candidates.every((event) => JSON.stringify(event.rules) === JSON.stringify(candidates[0]?.rules)))
    return candidates[0];
  return undefined;
}

export function explainEvents(
  entries: JournalEntry[],
  game: Game,
  view?: Pick<View, 'eventCatalog' | 'pendingEvent' | 'players'>,
): JournalEntry[] {
  if (!entries.some((entry) => entry.event)) return entries;
  const history = publicHistory(game);
  const result: JournalEntry[] = [];
  const triggers = entries.filter((entry) => entry.event);
  for (let i = 0; i < entries.length; i++) {
    const entry = entries[i];
    if (!entry.event) {
      result.push(entry);
      continue;
    }
    const end = entries.findIndex((e, j) => j > i && !!e.event);
    const following = entries.slice(i + 1, end === -1 ? undefined : end);
    // Attach only the incident's immediate resolution. Later uses of persistent
    // event abilities must stay at their own point in the timeline.
    const resolution: JournalEntry[] = [];
    for (const next of following) {
      const record = history.get(actionId(next));
      if (
        actionId(next) !== actionId(entry) &&
        record?.action.action &&
        typeof record.action.action === 'object' &&
        'Playing' in record.action.action
      )
        break;
      resolution.push(next);
    }
    const records = [...new Set([entry, ...resolution].map(actionId))].flatMap((id) => history.get(id) ?? []);
    const pendingInfo =
      entry === triggers.at(-1) && view?.pendingEvent
        ? view.eventCatalog?.find((e) => e.id === view.pendingEvent?.id && e.name === entry.title)
        : undefined;
    const info = pendingInfo ?? eventInfo(entry.title, records, view?.eventCatalog ?? []);
    entry.event.info = info;
    const outcomes = resolution.filter(
      (next) =>
        next.title.split(' · ')[0] === entry.title ||
        next.title === 'Barbarian reinforcements' ||
        (info?.baseEffect === 'Gold deposits.' && next.title === 'Gold deposits'),
    );
    entry.event.outcomes = outcomes;
    explainBarbarianGains(entry, game);
    for (const outcome of outcomes) entries.splice(entries.indexOf(outcome), 1);
    if (
      entry === triggers.at(-1) &&
      view?.pendingEvent &&
      (info?.id === view.pendingEvent.id ||
        view.eventCatalog?.find((e) => e.id === view.pendingEvent?.id)?.name === entry.title)
    ) {
      entry.event.pending = `Waiting for ${game.players.find((p) => p.id === view.pendingEvent?.player)?.civilization ?? 'a choice'}`;
    }
    if (info) addExplanations(entry, history.get(actionId(entry)), records, game, view);
    if (!outcomes.length && !entry.event.explanations.length && !entry.event.pending)
      entry.event.explanations.push({ text: 'No immediate changes recorded.' });
    entry.text = [
      entry.title,
      ...outcomes.map((e) => e.text),
      ...entry.event.explanations.map((e) =>
        [game.players.find((p) => p.id === e.player)?.civilization, e.text].filter(Boolean).join(': '),
      ),
      entry.event.pending,
    ]
      .filter(Boolean)
      .join(' · ');
    result.push(entry);
  }
  return result;
}

function addExplanations(
  entry: JournalEntry,
  record: RecordAtAction | undefined,
  resolution: RecordAtAction[],
  game: Game,
  view?: Pick<View, 'players'>,
) {
  const event = entry.event!;
  const info = event.info!;
  if (!record) return;
  const actor =
    entry.event?.triggeredBy ??
    record.action.items?.find((item) => item.Advance?.take_incident_token)?.player ??
    record.actor;
  const targets = info.targets.includes('all')
    ? game.players.filter((p) => view?.players.some((publicPlayer) => publicPlayer.index === p.id))
    : game.players.filter((p) => p.id === actor);
  for (const player of targets) {
    const facts = record.facts.get(player.id);
    if (!facts) continue;
    const changes = resolution
      .flatMap((r) => r.action.items ?? [])
      .filter((item) => item.player === player.id && item.origin?.Incident === info.id);
    const protection =
      info.protectionAdvance &&
      (facts.advances.has(info.protectionAdvance) || facts.borrowed === displayName(info.protectionAdvance))
        ? displayName(info.protectionAdvance)
        : undefined;
    const special = view?.players
      .find((p) => p.index === player.id)
      ?.civilizationAdvances?.find((a) => a.id === info.protectionSpecialAdvance);
    const protectedBy =
      protection ?? (special?.prerequisites.some((a) => facts.advances.has(a.id)) ? special.name : undefined);
    if (protectedBy) {
      event.explanations.push({
        player: player.id,
        text: `Protected by ${protectedBy}`,
        protected: true,
      });
    } else if (info.id === 51 && !facts.advances.has('Storage') && facts.borrowed !== 'Storage') {
      event.explanations.push({ player: player.id, text: 'Unaffected · no Storage' });
    } else if (
      info.name === 'Epidemics' &&
      !changes.some((item) => item.Units?.balance === 'Loss') &&
      !event.outcomes.some(
        (e) => e.player === player.id && e.tokens.some((t) => t.icon === 'unit' && t.tone === 'loss'),
      )
    ) {
      if (facts.units < 2 && facts.units >= 0)
        event.explanations.push({
          player: player.id,
          text: `${facts.units} ${facts.units === 1 ? 'unit' : 'units'} · requires 2+`,
        });
      else if (!event.pending && facts.units >= 2)
        event.explanations.push({ player: player.id, text: `${facts.units} units · no loss recorded` });
    } else if (
      info.minimumCities &&
      facts.cities >= 0 &&
      facts.cities < info.minimumCities &&
      !changes.some((item) => item.Structure?.balance === 'Loss')
    ) {
      event.explanations.push({
        player: player.id,
        text: `${info.name === 'Pestilence' ? 'No mood loss · ' : ''}${facts.cities} ${facts.cities === 1 ? 'city' : 'cities'} · requires ${info.minimumCities}+`,
      });
    }
  }
}
