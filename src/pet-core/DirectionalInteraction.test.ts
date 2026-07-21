import { describe, expect, it } from "vitest";

import { resolveDragDirection, resolveLookDirection } from "./DirectionalInteraction";

const directions = [
  { degrees: 0, label: "up", row: 9, column: 0 },
  { degrees: 90, label: "right", row: 9, column: 4 },
  { degrees: 180, label: "down", row: 10, column: 0 },
  { degrees: 270, label: "left", row: 10, column: 4 },
];
const windowRect = {
  position: { x: 100, y: 100 },
  size: { width: 144, height: 156 },
  scaleFactor: 1,
};

describe("directional interactions", () => {
  it("resolves horizontal drag direction with a jitter threshold", () => {
    expect(resolveDragDirection(100, 96)).toBe("running-left");
    expect(resolveDragDirection(100, 104)).toBe("running-right");
    expect(resolveDragDirection(100, 101)).toBeUndefined();
  });

  it("maps cursor positions around the pet to cardinal directions", () => {
    expect(resolveLookDirection({ x: 172, y: 50 }, windowRect, directions)?.label).toBe("up");
    expect(resolveLookDirection({ x: 300, y: 178 }, windowRect, directions)?.label).toBe("right");
    expect(resolveLookDirection({ x: 172, y: 300 }, windowRect, directions)?.label).toBe("down");
    expect(resolveLookDirection({ x: 20, y: 178 }, windowRect, directions)?.label).toBe("left");
  });

  it("does not look at a cursor inside the pet or outside the activation radius", () => {
    expect(resolveLookDirection({ x: 172, y: 178 }, windowRect, directions)).toBeUndefined();
    expect(resolveLookDirection({ x: 1_000, y: 1_000 }, windowRect, directions)).toBeUndefined();
  });
});
