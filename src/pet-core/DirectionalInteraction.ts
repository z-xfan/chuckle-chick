import type { PetAtlas } from "@/pet-assets/types";

export type DragDirection = "running-left" | "running-right";

export interface PhysicalPoint {
  x: number;
  y: number;
}

export interface PhysicalRect {
  position: PhysicalPoint;
  size: { width: number; height: number };
  scaleFactor: number;
}

export function resolveDragDirection(
  previousX: number,
  currentX: number,
  threshold = 2,
): DragDirection | undefined {
  const delta = currentX - previousX;
  if (Math.abs(delta) < threshold) return undefined;
  return delta < 0 ? "running-left" : "running-right";
}

export function resolveLookDirection(
  cursor: PhysicalPoint,
  windowRect: PhysicalRect,
  directions: PetAtlas["lookDirections"],
  radiusLogical = 320,
): PetAtlas["lookDirections"][number] | undefined {
  if (directions.length === 0) return undefined;

  const { position, size } = windowRect;
  const insideWindow =
    cursor.x >= position.x &&
    cursor.x <= position.x + size.width &&
    cursor.y >= position.y &&
    cursor.y <= position.y + size.height;
  if (insideWindow) return undefined;

  const centerX = position.x + size.width / 2;
  const centerY = position.y + size.height / 2;
  const deltaX = cursor.x - centerX;
  const deltaY = cursor.y - centerY;
  const scaleFactor = Math.max(windowRect.scaleFactor, 0.1);
  const distanceLogical = Math.hypot(deltaX, deltaY) / scaleFactor;
  if (distanceLogical > radiusLogical) return undefined;

  const degrees = (Math.atan2(deltaX, -deltaY) * 180) / Math.PI;
  const normalizedDegrees = (degrees + 360) % 360;
  return directions.reduce((closest, candidate) =>
    angularDistance(candidate.degrees, normalizedDegrees) <
    angularDistance(closest.degrees, normalizedDegrees)
      ? candidate
      : closest,
  );
}

function angularDistance(first: number, second: number): number {
  const difference = Math.abs(first - second) % 360;
  return Math.min(difference, 360 - difference);
}
