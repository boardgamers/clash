import { activeHistory } from './active-history.ts';
import { activeCombat, type ActiveCombat } from './active-combat.ts';
import { combatJournal, type CombatRound } from './combat-journal.ts';
import { journal } from './journal.ts';
import type { Game, JournalEntry, View } from './types.ts';

export const BATTLE_DURATION = 4400;
export interface BattleCue {
  key: string;
  combat: CombatRound;
  location: ActiveCombat | null;
  roll: boolean;
  losses: { civilization: string; label: string; value: string }[];
}

/** Cut off the log before formatting, so future rolls, cards and casualties cannot leak into a replay. */
export function historyThrough(game: Game, cursor: number): Game {
  let remaining = Math.max(0, cursor);
  const log = activeHistory(game).flatMap((age) => {
    const rounds = age.rounds.flatMap((round) => {
      const turns = round.turns.flatMap((turn) => {
        if (!remaining) return [];
        const actions = (turn.actions ?? []).slice(0, remaining);
        remaining -= actions.length;
        return [{ ...turn, actions }];
      });
      return turns.length ? [{ ...round, turns }] : [];
    });
    return rounds.length ? [{ ...age, rounds }] : [];
  });
  return { ...game, log, log_index: log.at(-1)?.rounds.at(-1)?.turns.at(-1)?.actions?.length ?? 0 };
}
const flatten = (game: Game) =>
  activeHistory(game).flatMap((a) => a.rounds.flatMap((r) => r.turns.flatMap((t) => t.actions ?? [])));
export const battleCursor = (game: Game) => flatten(game).length;
const rounds = (entries: JournalEntry[]): JournalEntry[] =>
  entries.flatMap((entry) => [...(entry.combat ? [entry] : []), ...rounds(entry.event?.outcomes ?? [])]);

/** A cue is emitted only when this interval adds public battle information. Works with legacy and structured logs. */
export function battleCues(
  game: Game,
  after: number,
  through = flatten(game).length,
  view?: View | null,
): BattleCue[] {
  if (through <= after) return [];
  const prefix = historyThrough(game, through);
  const actions = flatten(prefix);
  const changed = actions.slice(after);
  if (
    !changed.some(
      (a) =>
        a.combat_stats ||
        a.items?.some(
          (i) =>
            i.CombatRound || i.CombatRoll || i.origin?.Ability === 'Combat' || i.Text === 'wins the battle',
        ) ||
        a.log?.some((l) => /Combat|Tactics|wins|battle/.test(l)),
    )
  )
    return [];
  const old = new Map(
    rounds(combatJournal(journal(historyThrough(game, after), view ?? undefined))).map((e) => [
      e.id,
      e.combat!,
    ]),
  );
  return rounds(combatJournal(journal(prefix, view ?? undefined))).flatMap((entry) => {
    const combat = entry.combat!;
    const previous = old.get(entry.id);
    if (JSON.stringify(previous) === JSON.stringify(combat)) return [];
    // Public completed stats or a recorded pending battle provide exact locations.
    // Never consult an action after this frame merely to find its later result.
    const matching = (s: ActiveCombat | null | undefined): s is ActiveCombat =>
      !!s && s.attacker?.player === combat.attacker.player && s.defender?.player === combat.defender.player;
    const ids = entry.id.split('-').slice(0, 4).map(Number);
    let start = 0;
    for (let a = 0; a < prefix.log!.length; a++)
      for (let r = 0; r < prefix.log![a].rounds.length; r++)
        for (let t = 0; t < prefix.log![a].rounds[r].turns.length; t++) {
          const entries = prefix.log![a].rounds[r].turns[t].actions ?? [];
          if (a < ids[0] || (a === ids[0] && r < ids[1]) || (a === ids[0] && r === ids[1] && t < ids[2]))
            start += entries.length;
          else if (a === ids[0] && r === ids[1] && t === ids[2]) start += ids[3];
        }
    const stats = actions
      .slice(start)
      .map((a) => a.combat_stats as ActiveCombat | undefined)
      .find(matching);
    const frameCombat = [...(game.board_history?.frames ?? [])]
      .reverse()
      .find((f) => f.cursor <= through && f.cursor >= start && matching(f.combat))?.combat;
    const current = through === flatten(game).length ? activeCombat(game) : null;
    const publicLocation = stats ?? frameCombat ?? (matching(current) ? current : null);
    const location = publicLocation
      ? {
          round: combat.round ?? publicLocation.round,
          attacker: { player: publicLocation.attacker.player, position: publicLocation.attacker.position },
          defender: { player: publicLocation.defender.player, position: publicLocation.defender.position },
        }
      : null;
    const losses = combat.outcomes.flatMap((e) =>
      e.tokens
        .filter((t) => t.icon === 'unit' && t.tone === 'loss')
        .map((t) => ({
          civilization: e.civilization ?? '',
          label: t.label.replace(/ at [A-Z]+\d+$/, ''),
          value: t.value ?? '−1',
        })),
    );
    for (const side of [combat.attacker, combat.defender]) {
      for (const unit of side.units ?? []) {
        if (typeof unit.type === 'string') continue;
        const name = unit.type.Leader;
        if (
          combat.outcomes.some(
            (e) => e.player === side.player && e.notes.some((n) => n.startsWith(`Lose ${name} at `)),
          )
        )
          losses.push({ civilization: side.civilization ?? '', label: name, value: '−1' });
      }
    }
    return [
      {
        key: `${game.board_history?.id ?? 'battle'}:${through}:${entry.id}`,
        combat,
        location,
        roll:
          JSON.stringify(previous?.attacker.dice) !== JSON.stringify(combat.attacker.dice) ||
          JSON.stringify(previous?.defender.dice) !== JSON.stringify(combat.defender.dice),
        losses,
      },
    ];
  });
}
