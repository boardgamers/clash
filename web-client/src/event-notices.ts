import { publicActions } from './replay-actions.ts';
import type { Game, View, LoggedAction } from './types.ts';
export interface EventNotice { key: string; cursor: number; name: string; rules: string[] }
function triggered(action: LoggedAction): string[] {
  return [...new Set([
    ...(action.log ?? []).flatMap(line => line.match(/^A new game event has been triggered: (.+)$/)?.[1] ?? []),
    ...(action.items ?? []).flatMap(item => item.Text?.match(/^triggers the event (.+)$/)?.[1] ?? []),
  ])];
}
/** Read public trigger records, never the event deck or current private hand. */
export function eventNotices(game: Game, view: Pick<View,'eventCatalog' | 'pendingEvent'>, after: number, through = Infinity): EventNotice[] {
  const actions = publicActions(game), result: EventNotice[] = [];
  for (let index = Math.max(0,after); index < Math.min(actions.length,through); index++) {
    for (const [ordinal,name] of triggered(actions[index]).entries()) {
      const candidates = view.eventCatalog?.filter(e=>e.name===name) ?? [];
      const ids = new Set<unknown>();
      // Follow-up records may identify a same-name variant. Stop at the next
      // public event trigger so an unrelated later event cannot supply its rules.
      for (let j=index;j<actions.length;j++) {
        if (j>index && triggered(actions[j]).length) break;
        for (const item of actions[j].items??[]) if(item.origin?.Incident != null) ids.add(item.origin.Incident);
      }
      const exact = candidates.filter(e=>ids.has(e.id));
      const pending = through === Infinity && !actions.slice(index+1).some(a=>triggered(a).length) ? candidates.find(e=>e.id===view.pendingEvent?.id) : undefined;
      const event = exact.length===1 ? exact[0] : pending;
      // Old logs sometimes omit the card ID. Show only rules shared by every
      // matching card rather than guessing which base-effect variant was drawn.
      const rules = event?.rules ?? candidates[0]?.rules.filter(rule=>candidates.every(e=>e.rules.includes(rule))) ?? [];
      result.push({key:`${index+1}:${ordinal}:${name}`,cursor:index+1,name,rules});
    }
  }
  return result;
}
export function liveEventNotices(before: Game | null, game: Game, view: Pick<View,'eventCatalog' | 'pendingEvent'>) {
  if (!before) {
    const event = view.eventCatalog?.find(e=>e.id===view.pendingEvent?.id);
    return event ? [{key:`pending:${event.id}`,cursor:publicActions(game).length,name:event.name,rules:event.rules}] : [];
  }
  return eventNotices(game,view,publicActions(before).length);
}
