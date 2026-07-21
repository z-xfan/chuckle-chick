import { afterEach, describe, expect, it, vi } from "vitest";

import { RandomActionScheduler } from "./RandomActionScheduler";

describe("RandomActionScheduler", () => {
  afterEach(() => vi.useRealTimers());

  it("plays an action after idle time and returns to idle", () => {
    vi.useFakeTimers();
    const played: string[] = [];
    const scheduler = new RandomActionScheduler(
      [{ name: "waving", durationMs: 400 }],
      (name) => played.push(name),
      { minIdleMs: 1_000, maxIdleMs: 1_000, random: () => 0 },
    );

    scheduler.start();
    vi.advanceTimersByTime(1_000);
    expect(played).toEqual(["waving"]);
    vi.advanceTimersByTime(400);
    expect(played).toEqual(["waving", "idle"]);
  });

  it("does not schedule actions while paused", () => {
    vi.useFakeTimers();
    const played: string[] = [];
    const scheduler = new RandomActionScheduler(
      [{ name: "jumping", durationMs: 300 }],
      (name) => played.push(name),
      { minIdleMs: 500, maxIdleMs: 500 },
    );

    scheduler.start();
    scheduler.pause();
    vi.advanceTimersByTime(2_000);
    expect(played).toEqual([]);
    scheduler.resume();
    expect(played).toEqual(["idle"]);
  });
});
