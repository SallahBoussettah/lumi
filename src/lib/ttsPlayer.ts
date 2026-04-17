import type { TtsClip } from "./tauri";

/**
 * Plays a queue of TTS clips back-to-back via Web Audio.
 *
 * - `enqueue(clip)` adds a clip; playback starts immediately if idle.
 * - `stop()` halts everything and drains the queue.
 * - `subscribe(cb)` notifies on every animation frame with the current
 *   playback position — the karaoke UI uses this to highlight words.
 *
 * Each clip is decoded from base64 WAV and scheduled on the same
 * AudioContext so adjacent clips chain seamlessly with no audible gap.
 */

export interface PlaybackState {
  /** Index of the currently-playing clip in the lifetime queue, or -1 if idle. */
  clipIndex: number;
  /** Milliseconds into the currently-playing clip. 0 if idle. */
  msInClip: number;
  /** Whether anything is currently playing. */
  playing: boolean;
}

type Listener = (state: PlaybackState) => void;

interface QueuedClip {
  clip: TtsClip;
  index: number;
  buffer: AudioBuffer;
  /** AudioContext.currentTime at which this clip started. */
  startedAt: number | null;
  source: AudioBufferSourceNode | null;
}

export class TtsPlayer {
  private ctx: AudioContext | null = null;
  private queue: QueuedClip[] = [];
  private nextIndex = 0;
  private currentIndex = -1;
  private listeners = new Set<Listener>();
  private rafId: number | null = null;

  private ensureCtx(): AudioContext {
    if (!this.ctx) {
      this.ctx = new (window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext })
          .webkitAudioContext)();
    }
    if (this.ctx.state === "suspended") {
      this.ctx.resume().catch(() => {});
    }
    return this.ctx;
  }

  async enqueue(clip: TtsClip): Promise<void> {
    const ctx = this.ensureCtx();
    const buffer = await this.decode(clip.audio_b64, ctx);
    const item: QueuedClip = {
      clip,
      index: this.nextIndex++,
      buffer,
      startedAt: null,
      source: null,
    };
    this.queue.push(item);
    this.scheduleNext();
  }

  private async decode(b64: string, ctx: AudioContext): Promise<AudioBuffer> {
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return await ctx.decodeAudioData(bytes.buffer);
  }

  /**
   * Schedule any clips in the queue that haven't been scheduled yet.
   * Each new clip starts at the previous clip's end so playback is gapless.
   */
  private scheduleNext() {
    if (!this.ctx) return;
    const ctx = this.ctx;
    let cursor = ctx.currentTime;

    // Find the last scheduled clip's end time.
    for (const q of this.queue) {
      if (q.startedAt !== null) {
        cursor = Math.max(cursor, q.startedAt + q.buffer.duration);
      }
    }

    for (const q of this.queue) {
      if (q.source) continue; // already scheduled
      const src = ctx.createBufferSource();
      src.buffer = q.buffer;
      src.connect(ctx.destination);
      const startAt = cursor;
      src.start(startAt);
      q.startedAt = startAt;
      q.source = src;
      src.onended = () => this.handleEnded(q.index);
      cursor = startAt + q.buffer.duration;
    }

    if (!this.rafId) {
      this.rafId = requestAnimationFrame(this.tick);
    }
  }

  private handleEnded = (index: number) => {
    // If this was the last clip, the next tick will see playing=false
    // and stop the RAF loop. Don't drop the clip from the queue — the
    // index sequence is the user's reference for which clip is current.
    if (this.currentIndex === index) {
      this.currentIndex = index + 1;
    }
  };

  private tick = () => {
    this.rafId = null;
    if (!this.ctx) return;
    const now = this.ctx.currentTime;

    // Find the clip we're inside right now
    let activeIndex = -1;
    let msInClip = 0;
    for (const q of this.queue) {
      if (q.startedAt === null) continue;
      const end = q.startedAt + q.buffer.duration;
      if (now >= q.startedAt && now < end) {
        activeIndex = q.index;
        msInClip = (now - q.startedAt) * 1000;
        break;
      }
    }

    const playing = activeIndex !== -1;
    if (playing) this.currentIndex = activeIndex;

    this.emit({
      clipIndex: activeIndex,
      msInClip,
      playing,
    });

    // Keep ticking while anything is scheduled and not yet finished.
    const stillPending = this.queue.some(
      (q) => q.startedAt !== null && q.startedAt + q.buffer.duration > now
    );
    if (stillPending) {
      this.rafId = requestAnimationFrame(this.tick);
    }
  };

  stop() {
    for (const q of this.queue) {
      if (q.source) {
        try {
          q.source.stop();
        } catch {
          /* ignore */
        }
        q.source.disconnect();
      }
    }
    this.queue = [];
    this.currentIndex = -1;
    // Reset the lifetime counter so the next batch of clips starts at index 0.
    // Consumers (e.g. VoiceMode) treat clipIndex as an offset into a per-turn
    // chunks array — without this reset, indices from prior turns would
    // overshoot and the karaoke counter would mark every word as spoken.
    this.nextIndex = 0;
    if (this.rafId) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
    this.emit({ clipIndex: -1, msInClip: 0, playing: false });
  }

  isPlaying(): boolean {
    if (!this.ctx) return false;
    const now = this.ctx.currentTime;
    return this.queue.some(
      (q) => q.startedAt !== null && q.startedAt + q.buffer.duration > now
    );
  }

  subscribe(cb: Listener): () => void {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }

  private emit(state: PlaybackState) {
    this.listeners.forEach((cb) => cb(state));
  }
}
