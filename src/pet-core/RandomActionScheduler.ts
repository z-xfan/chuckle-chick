export interface ScheduledPetAction {
  name: string;
  durationMs: number;
}

export interface RandomActionSchedulerOptions {
  minIdleMs?: number;
  maxIdleMs?: number;
  random?: () => number;
}

export class RandomActionScheduler {
  private timer?: ReturnType<typeof setTimeout>;
  private running = false;
  private paused = false;
  private readonly minIdleMs: number;
  private readonly maxIdleMs: number;
  private readonly random: () => number;

  constructor(
    private readonly actions: ScheduledPetAction[],
    private readonly play: (animationName: string) => void,
    options: RandomActionSchedulerOptions = {},
  ) {
    if (actions.length === 0) throw new Error("随机动作列表不能为空");
    this.minIdleMs = options.minIdleMs ?? 4_000;
    this.maxIdleMs = options.maxIdleMs ?? 8_000;
    this.random = options.random ?? Math.random;
  }

  start(): void {
    if (this.running) return;
    this.running = true;
    this.scheduleNextAction();
  }

  pause(): void {
    this.paused = true;
    this.clearTimer();
  }

  resume(): void {
    if (!this.running || !this.paused) return;
    this.paused = false;
    this.play("idle");
    this.scheduleNextAction();
  }

  stop(): void {
    this.running = false;
    this.paused = false;
    this.clearTimer();
  }

  private scheduleNextAction(): void {
    if (!this.running || this.paused) return;
    const delay = this.minIdleMs + this.random() * (this.maxIdleMs - this.minIdleMs);
    this.timer = setTimeout(() => this.playRandomAction(), delay);
  }

  private playRandomAction(): void {
    if (!this.running || this.paused) return;
    const index = Math.min(Math.floor(this.random() * this.actions.length), this.actions.length - 1);
    const action = this.actions[index]!;
    this.play(action.name);
    this.timer = setTimeout(() => {
      this.play("idle");
      this.scheduleNextAction();
    }, action.durationMs);
  }

  private clearTimer(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = undefined;
  }
}
