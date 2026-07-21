import { describe, expect, it } from "vitest";

import { AnimationTimeline } from "./AnimationTimeline";

describe("AnimationTimeline", () => {
  const animation = { row: 0, frames: 3, durationsMs: [100, 200, 300] };

  it("uses each frame's configured duration", () => {
    const timeline = new AnimationTimeline(animation);

    expect(timeline.advance(99)).toBe(0);
    expect(timeline.advance(1)).toBe(1);
    expect(timeline.advance(199)).toBe(1);
    expect(timeline.advance(1)).toBe(2);
  });

  it("handles a large tick and loops", () => {
    const timeline = new AnimationTimeline(animation);

    expect(timeline.advance(650)).toBe(0);
    expect(timeline.advance(50)).toBe(1);
  });

  it("ignores invalid or negative elapsed time", () => {
    const timeline = new AnimationTimeline(animation);

    expect(timeline.advance(-10)).toBe(0);
    expect(timeline.advance(Number.NaN)).toBe(0);
  });
});
