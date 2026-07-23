import type { PetAnimation } from "@/pet-assets/types";

export function resolveAnimationFrameColumns(animation: PetAnimation): number[] {
  return (
    animation.frameColumns?.slice() ??
    Array.from({ length: animation.frames }, (_, column) => column)
  );
}
