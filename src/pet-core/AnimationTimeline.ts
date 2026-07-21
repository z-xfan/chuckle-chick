import type { PetAnimation } from "@/pet-assets/types";

export class AnimationTimeline {
  private elapsedInFrameMs = 0;
  private frameIndex = 0;

  constructor(private readonly animation: PetAnimation) {}

  get currentFrame(): number {
    return this.frameIndex;
  }

  advance(deltaMs: number): number {
    if (!Number.isFinite(deltaMs) || deltaMs <= 0) return this.frameIndex;

    this.elapsedInFrameMs += deltaMs;
    while (this.elapsedInFrameMs >= this.currentDuration) {
      this.elapsedInFrameMs -= this.currentDuration;
      this.frameIndex = (this.frameIndex + 1) % this.animation.frames;
    }

    return this.frameIndex;
  }

  reset(): void {
    this.elapsedInFrameMs = 0;
    this.frameIndex = 0;
  }

  private get currentDuration(): number {
    return this.animation.durationsMs[this.frameIndex] ?? 1;
  }
}
