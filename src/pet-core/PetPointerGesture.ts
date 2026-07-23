export interface GesturePoint {
  x: number;
  y: number;
}

export type GestureEndResult = "click" | "drag" | "none";

export class PetPointerGesture {
  private pointerId?: number;
  private start?: GesturePoint;
  private dragging = false;

  constructor(private readonly dragThreshold = 4) {
    if (!Number.isFinite(dragThreshold) || dragThreshold < 0) {
      throw new Error("拖动阈值必须是非负数");
    }
  }

  begin(pointerId: number, point: GesturePoint): void {
    this.pointerId = pointerId;
    this.start = point;
    this.dragging = false;
  }

  move(pointerId: number, point: GesturePoint): boolean {
    if (pointerId !== this.pointerId || !this.start || this.dragging) return false;

    const deltaX = point.x - this.start.x;
    const deltaY = point.y - this.start.y;
    if (deltaX * deltaX + deltaY * deltaY < this.dragThreshold * this.dragThreshold) return false;

    this.dragging = true;
    return true;
  }

  end(pointerId: number): GestureEndResult {
    if (pointerId !== this.pointerId || !this.start) return "none";
    const result = this.dragging ? "drag" : "click";
    this.reset();
    return result;
  }

  cancel(pointerId?: number): void {
    if (pointerId !== undefined && pointerId !== this.pointerId) return;
    this.reset();
  }

  get isDragging(): boolean {
    return this.dragging;
  }

  get isTracking(): boolean {
    return this.pointerId !== undefined;
  }

  private reset(): void {
    this.pointerId = undefined;
    this.start = undefined;
    this.dragging = false;
  }
}
