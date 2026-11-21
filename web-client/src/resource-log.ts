import type { LoggedAction, Pile, Resource } from './types.ts';

type Item = NonNullable<LoggedAction['items']>[number];
const resources: Record<string, Resource> = {
  food: 'food', wood: 'wood', ore: 'ore', idea: 'ideas', ideas: 'ideas', gold: 'gold',
  'mood token': 'mood_tokens', 'mood tokens': 'mood_tokens',
  'culture token': 'culture_tokens', 'culture tokens': 'culture_tokens',
  captive: 'captives', captives: 'captives',
};

// Parse completed resource transactions, not offers, rules, conversions, or
// "Added" collection modifiers (which intentionally describe included effects).
function transaction(text: string): { balance: 'Pay' | 'Loss' | 'Gain'; pile: Pile; reason?: string } | undefined {
  const match = /^(Pay|Paid|Lose|Lost|Gain|Gained) (.+?)(?: with (.+))?$/.exec(text);
  if (!match) return;
  const pile: Pile = {};
  for (const part of match[2].split(/,\s*(?:and\s+)?|\s+and\s+|\s*\+\s*/)) {
    const quantity = /^(\d+) (.+)$/.exec(part);
    const resource = quantity && resources[quantity[2]];
    if (!quantity || !resource) return;
    pile[resource] = (pile[resource] ?? 0) + Number(quantity[1]);
  }
  return {
    balance: /^Pa/.test(match[1]) ? 'Pay' : /^Lo/.test(match[1]) ? 'Loss' : 'Gain',
    pile,
    reason: match[3] ? `With ${match[3]}` : undefined,
  };
}

/** Normalize only a trailing prose echo of the immediately preceding resource
 * record. Never merge resource records, search across intervening items/actions,
 * or guess from amounts alone. Legacy prose without its record remains intact.
 */
export function resourceExplanation(item: Item, previous?: Item): string | undefined {
  const text = item.Text;
  if (text === undefined || !previous?.Resources || !item.origin || !previous.origin ||
      item.player !== previous.player) return text;
  const origin = Object.entries(item.origin);
  if (!origin.length || origin.length !== Object.keys(previous.origin).length ||
      !origin.every(([key, value]) => previous.origin?.[key] === value)) return text;
  const parsed = transaction(text);
  if (!parsed) return text;
  const actual = previous.Resources;
  // These two audited payment listeners explicitly call the payment a loss.
  // Do not equate Pay and Loss for unrelated origins/transactions.
  const paymentLoss = actual.balance === 'Pay' && parsed.balance === 'Loss' &&
    (item.origin.Incident === 49 || item.origin.Ability === 'Viking Raids');
  if (actual.balance !== parsed.balance && !paymentLoss) return text;
  const keys = new Set([...Object.keys(actual.resources), ...Object.keys(parsed.pile)]);
  if (![...keys].every(key => (actual.resources[key as Resource] ?? 0) === (parsed.pile[key as Resource] ?? 0))) return text;
  return parsed.reason;
}
