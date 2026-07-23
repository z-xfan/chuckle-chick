import { describe, expect, it } from "vitest";

import { PetPointerGesture } from "./PetPointerGesture";

describe("PetPointerGesture", () => {
  it("treats a release below the movement threshold as a click", () => {
    const gesture = new PetPointerGesture(4);
    gesture.begin(1, { x: 10, y: 20 });

    expect(gesture.move(1, { x: 12, y: 22 })).toBe(false);
    expect(gesture.end(1)).toBe("click");
  });

  it("starts dragging when movement reaches the threshold", () => {
    const gesture = new PetPointerGesture(4);
    gesture.begin(1, { x: 10, y: 20 });

    expect(gesture.move(1, { x: 14, y: 20 })).toBe(true);
    expect(gesture.move(1, { x: 30, y: 20 })).toBe(false);
    expect(gesture.end(1)).toBe("drag");
  });

  it("ignores events from another pointer", () => {
    const gesture = new PetPointerGesture();
    gesture.begin(7, { x: 0, y: 0 });

    expect(gesture.move(8, { x: 20, y: 0 })).toBe(false);
    expect(gesture.end(8)).toBe("none");
    expect(gesture.end(7)).toBe("click");
  });

  it("clears an interrupted gesture", () => {
    const gesture = new PetPointerGesture();
    gesture.begin(1, { x: 0, y: 0 });
    gesture.cancel(1);

    expect(gesture.isTracking).toBe(false);
    expect(gesture.end(1)).toBe("none");
  });
});
