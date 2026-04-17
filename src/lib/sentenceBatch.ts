/**
 * Streaming sentence batcher for voice mode.
 *
 * Push token deltas as they arrive; the batcher emits each completed sentence
 * to the callback the moment it sees a terminator (.!?) followed by whitespace
 * or end-of-input. Use `flush()` when the stream ends to drain any remainder.
 *
 * Sentences are passed through `cleanForTts` to strip markdown noise so TTS
 * doesn't read "asterisk asterisk bold asterisk asterisk".
 */

export type SentenceCallback = (sentence: string, index: number) => void;

const SENTENCE_RE = /^([\s\S]*?[.!?])(\s+|$)/;

export class SentenceBatcher {
  private buf = "";
  private index = 0;

  constructor(private onSentence: SentenceCallback) {}

  push(delta: string): void {
    this.buf += delta;
    while (true) {
      const m = this.buf.match(SENTENCE_RE);
      if (!m) break;
      const raw = m[1].trim();
      this.buf = this.buf.slice(m[0].length);
      const cleaned = cleanForTts(raw);
      if (cleaned) this.onSentence(cleaned, this.index++);
    }
  }

  flush(): void {
    const rest = this.buf.trim();
    this.buf = "";
    if (!rest) return;
    const cleaned = cleanForTts(rest);
    if (cleaned) this.onSentence(cleaned, this.index++);
  }

  reset(): void {
    this.buf = "";
    this.index = 0;
  }
}

/** Strip markdown formatting characters that would sound bad in TTS. */
export function cleanForTts(text: string): string {
  return text
    .replace(/```[\s\S]*?```/g, " code block ")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/\*([^*]+)\*/g, "$1")
    .replace(/__([^_]+)__/g, "$1")
    .replace(/_([^_]+)_/g, "$1")
    .replace(/~~([^~]+)~~/g, "$1")
    .replace(/^#+\s+/gm, "")
    .replace(/^>\s+/gm, "")
    .replace(/^[-*+]\s+/gm, "")
    .replace(/^\d+\.\s+/gm, "")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/!\[[^\]]*\]\([^)]+\)/g, "")
    .replace(/\s+/g, " ")
    .trim();
}
