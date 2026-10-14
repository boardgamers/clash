import type { Move } from './types.ts';

export type SoundCue =
  'hover' | 'select' | 'collect' | 'build' | 'research' | 'draw' | 'move' | 'confirm' | 'undo' | 'error' | 'battle' | 'casualty' | 'sink';
// Keep action cues audible without making repeated map hovers intrusive.
const MASTER_GAIN = 0.4;
const notes: Record<SoundCue, number[]> = {
  hover: [560],
  select: [420],
  collect: [330, 440],
  build: [165, 220, 330],
  research: [660, 880, 990],
  draw: [523, 784, 1047],
  move: [220, 294],
  confirm: [392, 494],
  undo: [330, 220],
  error: [180, 160],
  battle: [110, 165, 110, 220],
  casualty: [294, 196, 98],
  sink: [220, 165, 110, 55],
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
  private lastHover = -Infinity;
  private soundGeneration = 0;
  private createContext: () => AudioContext;
  constructor(createContext: () => AudioContext = () => new AudioContext()) {
    this.createContext = createContext;
  }

  setEnabled(enabled: boolean) {
    this.enabled = enabled;
    if (!enabled) this.stop();
    if (this.context) this.volume?.gain.setValueAtTime(enabled ? MASTER_GAIN : 0, this.context.currentTime);
  }

  unlock() {
    if (!this.enabled || this.disposed) return;
    try {
      if (!this.context) {
        this.context = this.createContext();
        this.volume = this.context.createGain();
        this.volume.gain.value = MASTER_GAIN;
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
      (typeof document !== 'undefined' && document.hidden)
    )
      return;
    if (context.state === 'suspended') {
      if (cue !== 'hover') {
        const generation = this.soundGeneration;
        void context.resume().then(() => {
          if (context.state === 'running' && generation === this.soundGeneration) this.play(cue);
        }).catch(() => {});
      }
      return;
    }
    if (context.state !== 'running') return;
    if (this.voices.size >= 12) {
      if (cue === 'hover') return;
      this.stop();
    }
    if (cue === 'hover') {
      if (context.currentTime - this.lastHover < 0.09) return;
      this.lastHover = context.currentTime;
    }
    const duration =
      cue === 'hover' ? 0.055 : cue === 'select' ? 0.075 : cue === 'sink' ? 0.32 : cue === 'casualty' ? 0.22 : cue === 'research' || cue === 'draw' ? 0.24 : 0.18;
    notes[cue].forEach((frequency, index) => {
      const start = context.currentTime + index * (cue === 'research' ? 0.075 : 0.055);
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.type = ['build', 'move', 'battle', 'casualty', 'sink'].includes(cue) ? 'triangle' : 'sine';
      oscillator.frequency.setValueAtTime(frequency, start);
      oscillator.frequency.exponentialRampToValueAtTime(frequency * (cue === 'sink' ? 0.4 : cue === 'casualty' ? 0.65 : 0.96), start + duration);
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(
        cue === 'hover' ? 0.18 : cue === 'select' ? 0.25 : 0.4,
        start + 0.006,
      );
      if (cue !== 'hover') {
        gain.gain.setValueAtTime(cue === 'select' ? 0.25 : 0.4, start + duration * 0.35);
      }
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
    this.soundGeneration++;
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
