import { describe, expect, it } from "vitest";

import { SpeechBubbleQueue } from "./SpeechBubbleQueue";

describe("SpeechBubbleQueue", () => {
  it("activates the first message and queues later messages", () => {
    const queue = new SpeechBubbleQueue();

    expect(queue.enqueue("第一条", undefined, 1_000)).toBe("accepted");
    expect(queue.enqueue("第二条", 5_000, 2_000)).toBe("accepted");
    expect(queue.current?.message).toBe("第一条");
    expect(queue.waitingCount).toBe(1);
    expect(queue.finishCurrent()).toEqual({ message: "第二条", durationMs: 5_000 });
  });

  it("deduplicates the same message for thirty seconds", () => {
    const queue = new SpeechBubbleQueue();
    queue.enqueue("你好", undefined, 10_000);

    expect(queue.enqueue("你好", undefined, 39_999)).toBe("duplicate");
    expect(queue.enqueue("你好", undefined, 40_000)).toBe("accepted");
  });

  it("keeps at most three waiting messages and drops the oldest", () => {
    const queue = new SpeechBubbleQueue();
    queue.enqueue("当前", undefined, 0);
    queue.enqueue("等待一", undefined, 1);
    queue.enqueue("等待二", undefined, 2);
    queue.enqueue("等待三", undefined, 3);
    queue.enqueue("等待四", undefined, 4);

    expect(queue.waitingCount).toBe(3);
    queue.finishCurrent();
    expect(queue.current?.message).toBe("等待二");
  });

  it("clamps duration and rejects blank messages", () => {
    const queue = new SpeechBubbleQueue();

    expect(queue.enqueue("  ", 1_000)).toBe("empty");
    queue.enqueue("短", 1_000);
    expect(queue.current?.durationMs).toBe(2_000);
    queue.finishCurrent();
    queue.enqueue("长", 20_000);
    expect(queue.current?.durationMs).toBe(8_000);
  });
});
