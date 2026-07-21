import type { PhysicalPoint, PhysicalRect } from "@/pet-core/DirectionalInteraction";

export interface PointerSnapshot {
  cursor: PhysicalPoint;
  windowRect: PhysicalRect;
}

export type StopObserving = () => void;

export async function startPetWindowDragging(): Promise<void> {
  if (!("__TAURI_INTERNALS__" in window)) return;

  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().startDragging();
}

export async function observePetWindowMovement(
  listener: (position: PhysicalPoint) => void,
): Promise<StopObserving> {
  if (!("__TAURI_INTERNALS__" in window)) return () => undefined;

  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().onMoved(({ payload }) => listener(payload));
}

export async function getPointerSnapshot(): Promise<PointerSnapshot | undefined> {
  if (!("__TAURI_INTERNALS__" in window)) return undefined;

  const { invoke } = await import("@tauri-apps/api/core");
  return (await invoke<PointerSnapshot | null>("get_pointer_snapshot")) ?? undefined;
}

export async function isLeftMouseButtonPressed(): Promise<boolean | undefined> {
  if (!("__TAURI_INTERNALS__" in window)) return undefined;

  const { invoke } = await import("@tauri-apps/api/core");
  return (await invoke<boolean | null>("is_left_mouse_button_pressed")) ?? undefined;
}
