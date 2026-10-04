import type { JournalEntry } from './types.ts';

export interface CombatDie {
  value: number;
  symbol?: 'Infantry' | 'Cavalry' | 'Elephant' | 'Leader';
  effect?: string;
}
export interface CombatSide {
  player?: number;
  civilization?: string;
  units?: { type: string | { Leader: string }; count: number; label: string }[];
  dice?: CombatDie[];
  value?: number;
  hits?: number;
  cancelledHits?: { before: number; reasons: string[] };
  modifiers: string[];
  tactics?: string;
}
export interface CombatRound {
  round?: number;
  attacker: CombatSide;
  defender: CombatSide;
  result?: string;
  outcomes: JournalEntry[];
}

const cancelsHits = (text: string) =>
  /\b(?:cancels?|cancelled|canceled|ignores?|ignored|blocks?|blocked) (?:one|a|\d+|the first) hits?\b/i.test(
    text,
  );

function explainCancelledHits(side: CombatSide, opponent: CombatSide) {
  if (side.value === undefined || side.hits === undefined || !opponent.units) return;
  const reasons = [
    ...(opponent.dice
      ?.filter((die) => die.effect === '-1 hits, no combat value')
      .map(() => 'Elephant blocks 1 hit') ?? []),
    ...opponent.modifiers.filter(cancelsHits),
  ];
  const fighters = opponent.units.reduce((n, unit) => n + (unit.type === 'Settler' ? 0 : unit.count), 0);
  const before = Math.min(Math.floor(side.value / 5), fighters);
  // The logged hit count already includes these cancellations. Explain the reduction;
  // never subtract it again or mistake the enemy unit cap for a cancelled hit.
  if (reasons.length && before > side.hits) side.cancelledHits = { before, reasons };
}

function unitCounts(text: string): NonNullable<CombatSide['units']> {
  if (text === 'no units') return [];
  return text.split(/,\s*| and /).map((label) => {
    const match = label.match(/^(\d+) (settlers?|infantry|cavalry|elephants?|ships?|leaders?)$/i);
    if (!match) return { type: { Leader: label }, count: 1, label };
    const name = match[2].replace(/s$/, '');
    const type = name[0].toUpperCase() + name.slice(1);
    return { type: type === 'Leader' ? { Leader: 'Leader' } : type, count: Number(match[1]), label };
  });
}

function parseDice(text: string): CombatDie[] | null {
  if (text === 'no dice') return [];
  const parts = text.split(/,\s*(?![^()]*\))/);
  const dice: CombatDie[] = [];
  for (const part of parts) {
    const match = part.match(/^(\d+)(?: \((infantry|cavalry|elephant|leader) (?:face|die symbol): (.+)\))?$/);
    if (!match) return null; // Leave unfamiliar engine messages readable as prose.
    dice.push({
      value: Number(match[1]),
      ...(match[2]
        ? {
            symbol: (match[2][0].toUpperCase() + match[2].slice(1)) as CombatDie['symbol'],
            ...(match[3] === 'no bonus' ? {} : { effect: match[3] }),
          }
        : {}),
    });
  }
  return dice;
}

// Presentation only: collect public log lines across a round's prompts. The
// original journal remains intact for exports, history and undo.
export function combatJournal(entries: JournalEntry[]): JournalEntry[] {
  const output: JournalEntry[] = [];
  let current: JournalEntry | undefined;
  let turn = '';
  const start = (entry: JournalEntry, round?: number) => {
    current = {
      ...entry,
      id: entry.id + '-battle',
      kind: 'combat',
      title: round ? `Battle · Round ${round}` : 'Battle',
      player: undefined,
      civilization: undefined,
      tokens: [],
      notes: [],
      combat: { round, attacker: { modifiers: [] }, defender: { modifiers: [] }, outcomes: [] },
    };
    output.push(current);
  };
  const latest = () => {
    const index = output.indexOf(current!);
    if (index >= 0) output.splice(index, 1);
    output.push(current!);
  };
  for (const original of entries) {
    const entry = original.event
      ? { ...original, event: { ...original.event, outcomes: combatJournal(original.event.outcomes) } }
      : original;
    const key = entry.id.split('-').slice(0, 3).join('-');
    if (key !== turn) {
      current = undefined;
      turn = key;
    }
    const marker = entry.notes.length === 1 && entry.notes[0].match(/^Combat round (\d+)$/);
    if (marker) {
      start(entry, Number(marker[1]));
      continue;
    }
    const result =
      entry.notes.length === 1 &&
      entry.notes[0].match(/^(Attacker wins|Defender wins|Battle ends in a draw)$/);
    if (result && current?.combat) {
      current.combat.result =
        result[1] === 'Attacker wins'
          ? `${current.combat.attacker.civilization ?? 'Attacker'} wins`
          : result[1] === 'Defender wins'
            ? `${current.combat.defender.civilization ?? 'Defender'} wins`
            : 'Draw';
      latest();
      // Captures and casualty choices can follow the winner announcement.
      // Keep this battle until the next round marker or turn.
      continue;
    }
    if (entry.title === 'Tactics' && current?.combat) {
      const side = [current.combat.attacker, current.combat.defender].find((s) => s.player === entry.player);
      if (side) {
        const notes = entry.notes.filter((note) => {
          const revealed = note.match(/^Reveal Tactics Card (.+)$/);
          if (revealed) side.tactics = revealed[1];
          else if (note === 'Did not play a Tactics Card') side.tactics = 'None';
          else if (note === 'Play an action card face down') side.tactics = 'Face down';
          else return true;
          return false;
        });
        if (entry.tokens.length || notes.length) output.push({ ...entry, notes });
        latest();
        continue;
      }
    }
    if (entry.title !== 'Combat' || entry.player === undefined) {
      output.push(entry);
      continue;
    }
    let text = entry.notes.join(', ');
    const units = text.match(/^(Attacking|Defending) with (.+?)(?=, Roll |, Combat modifiers: |$)/);
    const roll = text.match(
      /(?:^|, )Roll (.+?) → combat value (\d+) → (\d+) hits? against (attacking|defending) units/,
    );
    const dice = roll ? parseDice(roll[1]) : null;
    if (!units && !dice && !current) {
      output.push(entry);
      continue;
    }
    const role = units
      ? units[1] === 'Attacking'
        ? 'attacker'
        : 'defender'
      : current?.combat?.attacker.player === entry.player
        ? 'attacker'
        : current?.combat?.defender.player === entry.player
          ? 'defender'
          : roll
            ? roll[4] === 'defending'
              ? 'attacker'
              : 'defender'
            : null;
    if (!role) {
      output.push(entry);
      continue;
    }
    if (!current) start(entry);
    const side = current!.combat![role];
    if (side.player !== undefined && side.player !== entry.player) {
      start(entry);
    }
    const target = current!.combat![role];
    target.player = entry.player;
    target.civilization = entry.civilization;
    if (units) {
      target.units = unitCounts(units[2]);
      text = text.replace(units[0], '');
    }
    if (roll && dice) {
      target.dice = dice;
      target.value = Number(roll[2]);
      target.hits = Number(roll[3]);
      text = text.replace(roll[0], '');
    }
    text = text.replace(/^,\s*|,\s*$/g, '');
    if (text.startsWith('Combat modifiers: ')) {
      const clauses = text.slice('Combat modifiers: '.length).split(/,\s*(?![^()]*\))/);
      // A journal entry can merge later captures into the same player's roll.
      // Only move known roll modifiers into the table; keep other effects below.
      const remainder: string[] = [];
      for (const clause of clauses) {
        if (/combat value|extra die/i.test(clause) || cancelsHits(clause)) target.modifiers.push(clause);
        else remainder.push(clause);
      }
      text = remainder.join(', ');
    }
    if (entry.tokens.length || text) current!.combat!.outcomes.push({ ...entry, notes: text ? [text] : [] });
    latest();
  }
  for (const entry of output) {
    if (!entry.combat) continue;
    explainCancelledHits(entry.combat.attacker, entry.combat.defender);
    explainCancelledHits(entry.combat.defender, entry.combat.attacker);
  }
  return output;
}
