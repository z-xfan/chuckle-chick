import { describe, expect, it } from "vitest";

import { parsePetAtlas, parsePetManifest } from "./loadPetAsset";

describe("pet asset validation", () => {
  it("accepts a v2 pet manifest", () => {
    expect(
      parsePetManifest({
        id: "test-pet",
        displayName: "测试宠物",
        description: "测试",
        spriteVersionNumber: 2,
        spritesheetPath: "spritesheet.png",
      }).id,
    ).toBe("test-pet");
  });

  it("rejects animation durations that do not match the frame count", () => {
    expect(() =>
      parsePetAtlas({
        version: 2,
        image: "spritesheet.png",
        canvas: { width: 20, height: 10 },
        grid: { columns: 2, rows: 1, cellWidth: 10, cellHeight: 10 },
        animations: { idle: { row: 0, frames: 2, durationsMs: [100] } },
        lookDirections: [],
      }),
    ).toThrow("帧数与 durationsMs 数量不一致");
  });

  it("rejects inconsistent grid dimensions", () => {
    expect(() =>
      parsePetAtlas({
        version: 2,
        image: "spritesheet.png",
        canvas: { width: 21, height: 10 },
        grid: { columns: 2, rows: 1, cellWidth: 10, cellHeight: 10 },
        animations: { idle: { row: 0, frames: 2, durationsMs: [100, 100] } },
        lookDirections: [],
      }),
    ).toThrow("图集宽度与网格列定义不一致");
  });
});
