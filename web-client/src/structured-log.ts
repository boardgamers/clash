import type { Game, LoggedAction, Pile, View } from './types.ts';
import { officialWonderText } from './wonder-names.ts';

type Units = Record<string, number | string>;
type Item = NonNullable<LoggedAction['items']>[number];
type Context = Pick<View, 'eventCatalog' | 'logOriginNames'>;
const words = (name: unknown): string =>
  typeof name === 'string'
    ? name.replace(/([a-z])([A-Z])/g, '$1 $2').replaceAll('_', ' ')
    : name && typeof name === 'object'
      ? Object.values(name).map(words).join(' ')
      : String(name ?? '');
const unitList = (units: Units): string =>
  Object.entries(units)
    .filter(([, count]) => !!count)
    .map(([type, count]) => (typeof count === 'number' ? `${count} ${type}` : words(count)))
    .join(' and ') || 'no units';
const pileText = (pile: Pile): string =>
  Object.entries(pile)
    .filter(([, amount]) => amount)
    .map(([resource, amount]) => `${amount} ${words(resource)}`)
    .join(' and ');
const actor = (game: Game, player: number): string =>
  game.players.find((p) => p.id === player)?.name ?? `Player ${player + 1}`;
function source(item: Item, context?: Context): string {
  const origin = item.origin ?? {};
  const label = context?.logOriginNames?.[JSON.stringify(origin)];
  if (label) return label;
  if (origin.Incident !== undefined)
    return (
      context?.eventCatalog?.find((event) => event.id === origin.Incident)?.name ?? `Event ${origin.Incident}`
    );
  if (origin.CivilCard !== undefined) return `Card ${origin.CivilCard}`;
  if (origin.TacticsCard !== undefined) return 'Tactics';
  return words(Object.values(origin)[0]);
}

// Keep the journal's presentation parsers at a single boundary. Engine records
// remain structured and immutable; legacy saved logs already contain prose.
function lines(game: Game, action: LoggedAction, context?: Context): string[] {
  if (action.log) return action.log;
  const result: string[] = [];
  const stats = action.combat_stats as { attacker?: { player?: number } } | undefined;
  let attacker = stats?.attacker?.player;
  const playing = action.action && typeof action.action === 'object' ? action.action.Playing : undefined;
  const collect =
    playing && typeof playing === 'object' && 'Collect' in playing
      ? (playing.Collect as { city_position: string })
      : undefined;
  const add = (player: number, origin: string, text: string) =>
    result.push(`${actor(game, player)}: ${origin}: ${text}`);
  if (collect && action.items?.length)
    add(action.player ?? action.items[0].player, 'Collect', `Use city ${collect.city_position}`);
  for (const item of action.items ?? []) {
    const origin = source(item, context);
    const write = (text: string) => add(item.player, origin, text);
    if (item.Text !== undefined) {
      const event = item.Text.match(/^triggers the event (.+)$/);
      if (event) result.push(`A new game event has been triggered: ${event[1]}`);
      else if (item.Text === 'wins the battle')
        result.push(item.player === attacker ? 'Attacker wins' : 'Defender wins');
      else if (item.Text === 'ends the battle in a draw') result.push('Battle ends in a draw');
      else write(item.Text);
    }
    if (item.Resources)
      write(
        `${item.Resources.balance === 'Gain' ? 'Gain' : item.Resources.balance === 'Pay' ? 'Pay' : 'Lose'} ${pileText(item.Resources.resources)}`,
      );
    if (item.Action)
      write(`${item.Action.balance === 'Gain' ? 'Gain' : 'Pay'} ${item.Action.amount ?? 1} actions`);
    if (item.Advance) {
      const advance = item.Advance;
      if (advance.balance === 'Gain') {
        const token = advance.incident_token;
        const taken = token && typeof token === 'object' ? token.Take : undefined;
        write(
          `Gain ${words(advance.advance)} ${taken === undefined ? 'without taking an event token' : `and take an event token (${taken === 0 ? 'triggering an incident' : taken})`}`,
        );
      } else write(`Lose advance ${words(advance.advance)}`);
    }
    if (item.Structure) {
      const structure = item.Structure;
      write(
        `${structure.balance === 'Gain' ? 'Gain' : 'Lose'} ${structure.structure === 'CityCenter' ? 'city' : words(structure.structure) + ' at'} ${structure.position}`,
      );
    }
    if (item.Units) {
      const units = item.Units;
      for (const [type, count] of Object.entries(units.units))
        if (count)
          write(
            `${units.balance === 'Gain' ? 'Gain' : 'Lose'} ${typeof count === 'number' ? `${count} ${count === 1 ? type.replace(/s$/, '') : type}` : words(count)}${units.position ? ` at ${units.position}` : ''}`,
          );
    }
    if (item.MoodChange) write(`City ${item.MoodChange.city} became ${item.MoodChange.mood}`);
    if (item.Move) {
      const move = item.Move;
      write(`Move ${unitList(move.units)} from ${move.start} to ${move.destination}`);
    }
    if (item.Explore)
      write(
        `Explore ${item.Explore.tiles.map(([position, terrain]) => `${position}: ${words(terrain)}`).join('; ')}`,
      );
    if (item.HandCard) {
      const { card, from, to } = item.HandCard;
      const type =
        card && typeof card === 'object' && 'ObjectiveCard' in card
          ? 'objective'
          : card && typeof card === 'object' && 'Wonder' in card
            ? 'wonder'
            : 'action';
      const hand = (location: unknown) =>
        !!location && typeof location === 'object' && ('Hand' in location || 'RevealedHand' in location);
      if (hand(to)) write(`Draw 1 ${type} card`);
      else if (to && typeof to === 'object' && 'CompleteObjective' in to)
        write(`Complete objective ${words(to.CompleteObjective)}`);
      else if (hand(from)) write(`Discard 1 ${type} card`);
      else write(`${words(to)} ${type} card`);
    }
    if (item.CombatRound) {
      const round = item.CombatRound;
      attacker = item.player;
      result.push(`Combat round ${round.round}`);
      add(item.player, 'Combat', `Attacking with ${unitList(round.attackers)}`);
      add(round.defending_player, 'Combat', `Defending with ${unitList(round.defenders)}`);
    }
    if (item.CombatRoll) {
      const roll = item.CombatRoll;
      const dice =
        roll.rolls
          .map((die) => {
            const unit = typeof die.unit_type === 'object' ? 'leader' : die.unit_type.toLowerCase();
            const effect = !die.bonus
              ? 'no bonus'
              : unit === 'elephant'
                ? '-1 hits, no combat value'
                : unit === 'leader'
                  ? 're-roll'
                  : `+${die.combat_bonus ?? (unit === 'infantry' ? 1 : 2)} combat value${die.combat_bonus === 3 ? ' (Horsemanship)' : ''}`;
            return `${die.value} (${unit} face: ${effect})`;
          })
          .join(', ') || 'no dice';
      add(
        item.player,
        'Combat',
        `Roll ${dice} → combat value ${roll.combat_value} → ${roll.hits} hits against ${item.player === attacker ? 'defending' : 'attacking'} units`,
      );
      if (roll.combat_modifiers?.length)
        add(item.player, 'Combat', `Combat modifiers: ${roll.combat_modifiers.join(', ')}`);
    }
    if (item.InfluenceCultureAttempt) {
      const attempt = item.InfluenceCultureAttempt;
      write(
        `Influence ${words(attempt.structure)} at ${attempt.position} from ${attempt.starting_city_position}`,
      );
    }
  }
  return result;
}

export function readableHistory(game: Game, context?: Context): Game {
  const inferred: Record<string, string> = {};
  let eventName: string | undefined;
  const playedCards = new Map<number, number>();
  for (const age of game.log ?? [])
    for (const round of age.rounds)
      for (const turn of round.turns)
        for (const action of turn.actions ?? [])
          for (const item of action.items ?? []) {
            const handCard = item.HandCard;
            if (
              handCard?.to === 'PlayToDiscardFaceDown' &&
              handCard.card &&
              typeof handCard.card === 'object' &&
              'ActionCard' in handCard.card &&
              typeof handCard.card.ActionCard === 'number'
            )
              playedCards.set(item.player, handCard.card.ActionCard);
            const tactics = item.Text?.match(/^Reveal Tactics Card (.+)$/);
            const cardId = playedCards.get(item.player);
            if (tactics && cardId) inferred[JSON.stringify({ TacticsCard: cardId })] = tactics[1];
            const trigger = item.Text?.match(/^triggers the event (.+)$/);
            if (trigger) eventName = trigger[1];
            else if (eventName && item.origin?.Incident !== undefined) {
              inferred[JSON.stringify(item.origin)] = eventName;
              eventName = undefined;
            }
          }
  context = { ...context, logOriginNames: { ...inferred, ...context?.logOriginNames } };
  return {
    ...game,
    log: game.log?.map((age) => ({
      ...age,
      rounds: age.rounds.map((round) => ({
        ...round,
        turns: round.turns.map((turn) => ({
          ...turn,
          turn_type:
            turn.turn_type && typeof turn.turn_type === 'object' && 'Setup' in turn.turn_type
              ? 'Setup'
              : turn.turn_type,
          actions: turn.actions?.map((action) => ({
            ...action,
            log: lines(game, action, context).map(officialWonderText),
          })),
        })),
      })),
    })),
  };
}
