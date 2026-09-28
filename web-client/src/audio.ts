import type { Move } from './types.ts';

export type SoundCue = 'select' | 'collect' | 'build' | 'research' | 'move' | 'confirm' | 'undo' | 'error';
const notes: Record<SoundCue, number[]> = {
  select: [420],
  collect: [330, 440],
  build: [165, 220, 330],
  research: [660, 880, 990],
  move: [220, 294],
  confirm: [392, 494],
  undo: [330, 220],
  error: [180, 160],
};

export function moveSound(move: Move | null): SoundCue {
  if (move === 'Undo') return 'undo';
  if (!move || typeof move !== 'object') return 'confirm';
  if ('Movement' in move) return 'move';
  const action = move.Playing;
  if (action && typeof action === 'object') {
    if ('Collect' in action) return 'collect';
    if ('Advance' in action) return 'research';
    if ('Construct' in action || 'FoundCity' in action || 'Recruit' in action) return 'build';
  }
  return 'confirm';
}

export class GameAudio {
  private context: AudioContext | undefined;
  private volume: GainNode | undefined;
  private voices = new Map<OscillatorNode, GainNode>();
  private enabled = false;
  private disposed = false;
  private createContext: () => AudioContext;
  constructor(createContext: () => AudioContext = () => new AudioContext()) {
    this.createContext = createContext;
  }

  setEnabled(enabled: boolean) {
    this.enabled = enabled;
    if (!enabled) this.stop();
    if (this.context) this.volume?.gain.setValueAtTime(enabled ? 0.13 : 0, this.context.currentTime);
  }

  unlock() {
    if (!this.enabled || this.disposed) return;
    try {
      if (!this.context) {
        this.context = this.createContext();
        this.volume = this.context.createGain();
        this.volume.gain.value = 0.13;
        this.volume.connect(this.context.destination);
      }
      if (this.context.state === 'suspended') void this.context.resume().catch(() => {});
    } catch {
      // Audio is optional when the browser or device cannot provide it.
    }
  }

  play(cue: SoundCue) {
    const context = this.context;
    if (
      !this.enabled ||
      this.disposed ||
      !context ||
      !this.volume ||
      context.state !== 'running' ||
      (typeof document !== 'undefined' && document.hidden)
    )
      return;
    if (this.voices.size >= 12) return;
    const duration = cue === 'select' ? 0.045 : cue === 'research' ? 0.24 : 0.12;
    notes[cue].forEach((frequency, index) => {
      const start = context.currentTime + index * (cue === 'research' ? 0.075 : 0.055);
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.type = cue === 'build' || cue === 'move' ? 'triangle' : 'sine';
      oscillator.frequency.setValueAtTime(frequency, start);
      oscillator.frequency.exponentialRampToValueAtTime(frequency * 0.96, start + duration);
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(cue === 'select' ? 0.25 : 0.4, start + 0.006);
      gain.gain.exponentialRampToValueAtTime(0.001, start + duration);
      oscillator.connect(gain);
      gain.connect(this.volume!);
      this.voices.set(oscillator, gain);
      oscillator.onended = () => {
        oscillator.disconnect();
        gain.disconnect();
        this.voices.delete(oscillator);
      };
      oscillator.start(start);
      oscillator.stop(start + duration + 0.01);
    });
  }

  private stop() {
    for (const [oscillator, gain] of this.voices) {
      oscillator.stop();
      oscillator.disconnect();
      gain.disconnect();
    }
    this.voices.clear();
  }

  destroy() {
    this.disposed = true;
    this.stop();
    if (this.context) void this.context.close().catch(() => {});
  }
}
