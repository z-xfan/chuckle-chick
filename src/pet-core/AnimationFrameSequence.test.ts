import { describe, expect, it } from "vitest";

import { resolveAnimationFrameColumns } from "./AnimationFrameSequence";

describe("animation frame sequences", () => {
  it("uses consecutive columns when no explicit sequence is configured", () => {
    expect(
      resolveAnimationFrameColumns({
        row: 0,
        frames: 3,
        durationsMs: [100, 100, 100],
      }),
    ).toEqual([0, 1, 2]);
  });

  it("preserves explicit repeated columns for a reversible transition", () => {
    expect(
      resolveAnimationFrameColumns({
        row: 5,
        frames: 5,
        durationsMs: [180, 180, 320, 180, 260],
        frameColumns: [0, 1, 2, 1, 0],
      }),
    ).toEqual([0, 1, 2, 1, 0]);
  });
});
