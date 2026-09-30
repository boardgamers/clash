import { activeHistory } from './active-history.ts';
import type { Game, JournalEntry, JournalToken, LoggedAction, Pile, Resource } from './types.ts';

type CityFacts = NonNullable<JournalEntry['collection']>['city'];
type Collect = { city_position: string; collections: { position: string; pile: Pile; times: number }[] };
const resources = new Set(['food', 'wood', 'ore', 'ideas', 'gold', 'mood_tokens', 'culture_tokens']);
const bundledSources = new Set(['Public Education', 'Rice Cultivation', 'Metallurgy', 'Canals']);

function collect(action: LoggedAction): Collect | undefined {
  const move = action.action;
  if (!move || typeof move !== 'object' || !move.Playing || typeof move.Playing !== 'object') return;
  return (move.Playing as { Collect?: Collect }).Collect;
}

// Read the city as it was before activation, not its present-day buildings or mood.
export function collectionCities(game: Game): Map<string, CityFacts> {
  const cities = new Map<string, { structures: Set<string>; mood: string }>();
  const result = new Map<string, CityFacts>();
  for (const [a, age] of activeHistory(game).entries())
    for (const [r, round] of age.rounds.entries())
      for (const [t, turn] of round.turns.entries())
        for (const [c, action] of (turn.actions ?? []).entries()) {
          const collection = collect(action);
          const city = collection && cities.get(collection.city_position);
          if (city && action.log?.length)
            result.set(`${a}-${r}-${t}-${c}`, {
              size: 1 + city.structures.size,
              mood: city.mood,
              structures: [
                'Settlement',
                ...[...city.structures].map((s) => s.split(':')[1].replace(/([a-z])([A-Z])/g, '$1 $2')),
              ],
            });
          for (const item of action.items ?? []) {
            const structure = item.Structure;
            if (structure) {
              const existing = cities.get(structure.position);
              if (structure.structure === 'CityCenter' && structure.balance === 'Gain') {
                if (!existing || /^(Setup|Found city)$/i.test(String(item.origin?.Ability)))
                  cities.set(structure.position, { structures: new Set(), mood: 'Neutral' });
              } else if (existing && typeof structure.structure === 'object' && structure.structure) {
                const [type, name] = Object.entries(structure.structure)[0] ?? [];
                if (type === 'Building' || type === 'Wonder') {
                  const key = `${type}:${name}`;
                  if (structure.balance === 'Gain') existing.structures.add(key);
                  else existing.structures.delete(key);
                }
              }
            }
            if (item.MoodChange) {
              const city = cities.get(item.MoodChange.city);
              if (city) city.mood = item.MoodChange.mood;
            }
          }
        }
  return result;
}

function tokenPile(tokens: JournalToken[]): Pile {
  const pile: Pile = {};
  for (const token of tokens)
    if (resources.has(token.icon) && token.value) {
      const resource = token.icon as Resource;
      pile[resource] = (pile[resource] ?? 0) + Number(token.value.replace('−', '-'));
    }
  return pile;
}

export function explainCollection(action: LoggedAction, entries: JournalEntry[], city?: CityFacts): void {
  const collection = collect(action);
  if (!collection || !action.log?.length) return;
  const entry = entries.find((e) => e.title.endsWith(` · ${collection.city_position}`));
  if (!entry) return;
  entry.collection = { city, tiles: collection.collections, effects: [] };
  const base: Pile = {};
  for (const tile of collection.collections)
    for (const [r, amount] of Object.entries(tile.pile)) {
      const resource = r as Resource;
      base[resource] = (base[resource] ?? 0) + amount! * tile.times;
    }
  const total = tokenPile(entry.tokens.filter((t) => t.tone === 'gain'));
  const effects = entries.filter(
    (e) =>
      e.player === entry.player &&
      bundledSources.has(e.title) &&
      e.tokens.length > 0 &&
      !e.notes.length &&
      e.tokens.every((t) => resources.has(t.icon)),
  );
  const bonus = tokenPile(effects.flatMap((e) => e.tokens));
  if (
    [...resources].some(
      (r) => (total[r as Resource] ?? 0) !== (base[r as Resource] ?? 0) + (bonus[r as Resource] ?? 0),
    )
  )
    return;
  // These logs describe additions already included in Collect's total, not a second gain.
  for (const effect of effects) {
    entry.collection.effects.push({ source: effect.title, tokens: effect.tokens });
    entries.splice(entries.indexOf(effect), 1);
  }
}
