export interface SpeechBubble {
  message: string;
  durationMs: number;
}

export type EnqueueResult = "accepted" | "duplicate" | "empty";

export interface SpeechBubbleQueueOptions {
  maxWaiting?: number;
  duplicateWindowMs?: number;
  defaultDurationMs?: number;
}

export class SpeechBubbleQueue {
  private activeBubble?: SpeechBubble;
  private readonly waiting: SpeechBubble[] = [];
  private readonly recentMessages = new Map<string, number>();
  private readonly maxWaiting: number;
  private readonly duplicateWindowMs: number;
  private readonly defaultDurationMs: number;

  constructor(options: SpeechBubbleQueueOptions = {}) {
    this.maxWaiting = options.maxWaiting ?? 3;
    this.duplicateWindowMs = options.duplicateWindowMs ?? 30_000;
    this.defaultDurationMs = options.defaultDurationMs ?? 4_000;
  }

  enqueue(message: string, durationMs?: number, now = Date.now()): EnqueueResult {
    const normalizedMessage = message.trim();
    if (!normalizedMessage) return "empty";

    this.removeExpiredHistory(now);
    const previousTime = this.recentMessages.get(normalizedMessage);
    if (previousTime !== undefined && now - previousTime < this.duplicateWindowMs) {
      return "duplicate";
    }

    const bubble = {
      message: normalizedMessage,
      durationMs: clampDuration(durationMs ?? this.defaultDurationMs),
    };
    this.recentMessages.set(normalizedMessage, now);

    if (!this.activeBubble) {
      this.activeBubble = bubble;
      return "accepted";
    }

    if (this.waiting.length >= this.maxWaiting) this.waiting.shift();
    this.waiting.push(bubble);
    return "accepted";
  }

  get current(): SpeechBubble | undefined {
    return this.activeBubble;
  }

  get waitingCount(): number {
    return this.waiting.length;
  }

  finishCurrent(): SpeechBubble | undefined {
    this.activeBubble = this.waiting.shift();
    return this.activeBubble;
  }

  clear(): void {
    this.activeBubble = undefined;
    this.waiting.length = 0;
  }

  private removeExpiredHistory(now: number): void {
    for (const [message, timestamp] of this.recentMessages) {
      if (now - timestamp >= this.duplicateWindowMs) this.recentMessages.delete(message);
    }
  }
}

function clampDuration(durationMs: number): number {
  if (!Number.isFinite(durationMs)) return 4_000;
  return Math.min(Math.max(Math.round(durationMs), 2_000), 8_000);
}
