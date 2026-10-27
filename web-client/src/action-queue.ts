import { predictMove, type PredictionEngine } from './prediction.ts';

export interface QueueEngine extends PredictionEngine {
  currentPlayer(state: string): number;
}
export interface QueuedAction { move: unknown; before: string; after: string; endTurn: boolean; group: number }
export type QueueResult = { ok: true } | { ok: false; reason: string };

/** Local planning only: never mutates the authoritative snapshot. */
export class ActionQueue {
  entries: QueuedAction[] = [];
  projected: string;
  readonly engine: QueueEngine;
  readonly seat: number;
  constructor(engine: QueueEngine, seat: number, base: string) {
    this.engine=engine;this.seat=seat;
    this.projected = engine.stripSecret(base, seat);
  }
  get count() { return new Set(this.entries.map(e => e.group)).size; }
  get ended() { return this.entries.at(-1)?.endTurn ?? false; }
  append(move: unknown): QueueResult {
    if (this.ended) return { ok: false, reason: 'Undo the queued end turn before adding another action.' };
    if (move === 'Undo' || move === 'Redo') return { ok: false, reason: 'Use Undo queued action.' };
    const before = this.projected;
    const previous=this.entries.at(-1);
    const group=previous && JSON.parse(before).events?.length ? previous.group : (previous?.group ?? 0)+1;
    const endTurn = typeof move === 'object' && move !== null && (move as {Playing?: unknown}).Playing === 'EndTurn';
    if (endTurn) {
      // Terminal intent only. Never simulate turn-start effects or status-phase draws.
      if (JSON.parse(before).events?.length) return { ok: false, reason: 'Finish your current choice before queuing end turn.' };
      this.entries.push({ move: structuredClone(move), before, after: before, endTurn, group });
      return { ok: true };
    }
    const prediction = predictMove(this.engine, before, move, this.seat);
    if (!('raw' in prediction)) return { ok: false, reason: 'This action cannot be queued safely. Wait for the earlier choice.' };
    if (this.engine.currentPlayer(prediction.raw) !== this.seat)
      return { ok: false, reason: 'This action needs another player’s decision. Wait for the earlier choice.' };
    this.entries.push({ move: structuredClone(move), before, after: prediction.raw, endTurn, group });
    this.projected = prediction.raw;
    return { ok: true };
  }
  undo() {
    const removed = this.entries.pop();
    if (removed) {
      this.projected = removed.before;
      while(this.entries.at(-1)?.group === removed.group) this.projected=this.entries.pop()!.before;
    }
  }
  /** Replay only the valid prefix, preserving the failed action and suffix for review. */
  replay(authoritative: string): { raw: string; applied: number; remaining: unknown[]; error?: string } {
    let raw = authoritative, applied = 0;
    for (const entry of this.entries) {
      try {
        if (this.engine.currentPlayer(raw) !== this.seat) throw Error('Another choice must resolve first.');
        if (!entry.endTurn) {
          const check = predictMove(this.engine, this.engine.stripSecret(raw, this.seat), entry.move, this.seat);
          if (!('raw' in check) || this.engine.currentPlayer(check.raw) !== this.seat)
            throw Error('The queued action is no longer safe or legal.');
        }
        raw = this.engine.tryMove(raw, JSON.stringify(entry.move), this.seat);
        applied++;
      } catch (error) {
        return { raw, applied, remaining: this.entries.slice(applied).map(e => e.move), error: String(error) };
      }
    }
    return { raw, applied, remaining: [] };
  }
}
