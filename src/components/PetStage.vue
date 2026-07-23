<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

import { loadPetAsset } from "@/pet-assets/loadPetAsset";
import type { PetAtlas } from "@/pet-assets/types";
import {
  resolveDragDirection,
  resolveLookDirection,
  type DragDirection,
} from "@/pet-core/DirectionalInteraction";
import { PetPointerGesture } from "@/pet-core/PetPointerGesture";
import { RandomActionScheduler } from "@/pet-core/RandomActionScheduler";
import { PixiPetRenderer } from "@/pet-renderer/PixiPetRenderer";
import {
  hideAssistantWindow,
  listenPetInteraction,
  toggleQuickPanel,
  type PetInteractionPayload,
} from "@/platform/assistant";
import {
  getPointerSnapshot,
  isLeftMouseButtonPressed,
  observePetWindowMovement,
  startPetWindowDragging,
  type StopObserving,
} from "@/platform/window";

const host = ref<HTMLDivElement>();
const errorMessage = ref("");
let renderer: PixiPetRenderer | undefined;
let scheduler: RandomActionScheduler | undefined;
let lookDirections: PetAtlas["lookDirections"] = [];
let stopObservingMovement: StopObserving | undefined;
let cursorTimer: number | undefined;
let dragEndTimer: number | undefined;
let dragButtonTimer: number | undefined;
let directActionTimer: number | undefined;
let dragButtonCheckInFlight = false;
let supportsDragButtonPolling: boolean | undefined;
let cursorCheckInFlight = false;
let isDragging = false;
let isDirectInteracting = false;
let isHovering = false;
let isLooking = false;
let lastWindowX: number | undefined;
let currentDragDirection: DragDirection | undefined;
let currentLookFrame = "";
let stopListeningForInteraction: (() => void) | undefined;
const pointerGesture = new PetPointerGesture(4);
const actionDurations = new Map<string, number>();
const directActionNames = ["waving", "review", "failed", "waiting", "running"];

onMounted(async () => {
  if (!host.value) return;

  try {
    const asset = await loadPetAsset("./pets/huangji-daxiao");
    renderer = new PixiPetRenderer(host.value);
    await renderer.initialize(asset);
    await renderer.play("idle");
    lookDirections = asset.atlas.lookDirections;
    const randomActions = ["waving", "review", "failed", "waiting", "running"]
      .map((name) => ({ name, animation: asset.atlas.animations[name] }))
      .filter((entry) => entry.animation)
      .map(({ name, animation }) => ({
        name,
        durationMs: animation!.durationsMs.reduce((total, duration) => total + duration, 0),
      }));
    for (const action of randomActions) actionDurations.set(action.name, action.durationMs);
    scheduler = new RandomActionScheduler(randomActions, (name) => {
      void renderer?.play(name);
    });
    scheduler.start();
    stopObservingMovement = await observePetWindowMovement(({ x }) => {
      if (!isDragging) return;
      if (supportsDragButtonPolling === false) scheduleDragEndFallback();
      if (lastWindowX === undefined) {
        lastWindowX = x;
        return;
      }
      const direction = resolveDragDirection(lastWindowX, x);
      lastWindowX = x;
      if (!direction || direction === currentDragDirection) return;
      currentDragDirection = direction;
      void renderer?.play(direction);
    });
    cursorTimer = window.setInterval(() => void updateCursorLook(), 80);
    stopListeningForInteraction = await listenPetInteraction((payload) => {
      void handleDirectInteractionRequest(payload);
    });
    window.addEventListener("blur", cancelPointerGesture);
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
});

onBeforeUnmount(() => {
  scheduler?.stop();
  stopObservingMovement?.();
  stopListeningForInteraction?.();
  if (cursorTimer !== undefined) window.clearInterval(cursorTimer);
  if (dragEndTimer !== undefined) window.clearTimeout(dragEndTimer);
  if (dragButtonTimer !== undefined) window.clearInterval(dragButtonTimer);
  if (directActionTimer !== undefined) window.clearTimeout(directActionTimer);
  window.removeEventListener("blur", cancelPointerGesture);
  renderer?.destroy();
});

function handlePointerDown(event: PointerEvent): void {
  if (isDragging || !event.isPrimary) return;
  pointerGesture.begin(event.pointerId, { x: event.clientX, y: event.clientY });
  event.currentTarget instanceof Element &&
    event.currentTarget.setPointerCapture?.(event.pointerId);
}

function handlePointerMove(event: PointerEvent): void {
  if (!event.isPrimary) return;
  const shouldStartDragging = pointerGesture.move(event.pointerId, {
    x: event.clientX,
    y: event.clientY,
  });
  if (!shouldStartDragging) return;
  if (
    event.currentTarget instanceof Element &&
    event.currentTarget.hasPointerCapture?.(event.pointerId)
  ) {
    event.currentTarget.releasePointerCapture(event.pointerId);
  }
  void beginDragging();
}

async function handlePointerUp(event: PointerEvent): Promise<void> {
  if (!event.isPrimary) return;
  const result = pointerGesture.end(event.pointerId);
  if (
    event.currentTarget instanceof Element &&
    event.currentTarget.hasPointerCapture?.(event.pointerId)
  ) {
    event.currentTarget.releasePointerCapture(event.pointerId);
  }
  if (result === "click") {
    await handlePetClick();
  } else if (result === "drag") {
    await finishDragging();
  }
}

async function handlePointerCancel(event: PointerEvent): Promise<void> {
  pointerGesture.cancel(event.pointerId);
  await finishDragging();
}

function cancelPointerGesture(): void {
  pointerGesture.cancel();
  if (isDragging) void finishDragging();
}

async function handlePetClick(): Promise<void> {
  try {
    const opened = await toggleQuickPanel();
    if (opened) await playDirectInteraction("waving");
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

async function beginDragging(): Promise<void> {
  if (isDragging) return;
  isDragging = true;
  stopDirectInteraction();
  lastWindowX = undefined;
  isLooking = false;
  currentLookFrame = "";
  currentDragDirection = undefined;
  supportsDragButtonPolling = undefined;
  scheduler?.pause();
  void hideAssistantWindow().catch(() => undefined);
  dragButtonTimer = window.setInterval(() => void checkDragButtonState(), 50);
  void checkDragButtonState();
  try {
    await startPetWindowDragging();
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
    await finishDragging();
  }
}

async function checkDragButtonState(): Promise<void> {
  if (!isDragging || dragButtonCheckInFlight) return;
  dragButtonCheckInFlight = true;
  try {
    const pressed = await isLeftMouseButtonPressed();
    supportsDragButtonPolling = pressed !== undefined;
    if (pressed === false) await finishDragging();
  } finally {
    dragButtonCheckInFlight = false;
  }
}

function scheduleDragEndFallback(): void {
  if (dragEndTimer !== undefined) window.clearTimeout(dragEndTimer);
  dragEndTimer = window.setTimeout(() => void finishDragging(), 180);
}

async function finishDragging(): Promise<void> {
  if (!isDragging) return;
  isDragging = false;
  if (dragEndTimer !== undefined) window.clearTimeout(dragEndTimer);
  dragEndTimer = undefined;
  if (dragButtonTimer !== undefined) window.clearInterval(dragButtonTimer);
  dragButtonTimer = undefined;
  supportsDragButtonPolling = undefined;
  lastWindowX = undefined;
  currentDragDirection = undefined;
  pointerGesture.cancel();
  if (isHovering) {
    await renderer?.play("jumping");
  } else {
    await renderer?.play("idle");
    scheduler?.resume();
  }
}

function handlePointerEnter(): void {
  isHovering = true;
  if (isDragging || isDirectInteracting) return;
  isLooking = false;
  currentLookFrame = "";
  scheduler?.pause();
  void renderer?.play("jumping");
}

function handlePointerLeave(): void {
  isHovering = false;
  if (isDragging || isDirectInteracting) return;
  isLooking = false;
  currentLookFrame = "";
  scheduler?.resume();
}

async function updateCursorLook(): Promise<void> {
  if (
    isDragging ||
    isDirectInteracting ||
    isHovering ||
    cursorCheckInFlight ||
    lookDirections.length === 0
  )
    return;
  cursorCheckInFlight = true;
  try {
    const snapshot = await getPointerSnapshot();
    if (isDragging || isDirectInteracting || isHovering) return;
    const direction = snapshot
      ? resolveLookDirection(snapshot.cursor, snapshot.windowRect, lookDirections)
      : undefined;

    if (!direction) {
      if (isLooking) {
        isLooking = false;
        currentLookFrame = "";
        scheduler?.resume();
      }
      return;
    }

    if (!isLooking) {
      isLooking = true;
      scheduler?.pause();
    }
    const frameKey = `${direction.row}:${direction.column}`;
    if (frameKey === currentLookFrame) return;
    currentLookFrame = frameKey;
    await renderer?.showLookDirection(direction.row, direction.column);
  } finally {
    cursorCheckInFlight = false;
  }
}

async function handleDirectInteractionRequest(payload: PetInteractionPayload): Promise<void> {
  const animationName =
    payload.animationName === "random"
      ? directActionNames[Math.floor(Math.random() * directActionNames.length)]!
      : payload.animationName;
  await playDirectInteraction(animationName);
}

async function playDirectInteraction(animationName: string): Promise<void> {
  if (isDragging) return;
  const durationMs = actionDurations.get(animationName);
  if (!durationMs) throw new Error(`未找到互动动作：${animationName}`);

  stopDirectInteraction();
  isDirectInteracting = true;
  isLooking = false;
  currentLookFrame = "";
  scheduler?.pause();
  await renderer?.play(animationName);
  directActionTimer = window.setTimeout(() => finishDirectInteraction(), durationMs);
}

function stopDirectInteraction(): void {
  if (directActionTimer !== undefined) window.clearTimeout(directActionTimer);
  directActionTimer = undefined;
  isDirectInteracting = false;
}

function finishDirectInteraction(): void {
  stopDirectInteraction();
  if (isDragging) return;
  if (isHovering) {
    void renderer?.play("jumping");
  } else {
    scheduler?.resume();
  }
}
</script>

<template>
  <div
    ref="host"
    class="pet-stage"
    role="img"
    aria-label="黄鸡大笑桌面宠物"
    @pointerdown.left="handlePointerDown"
    @pointermove="handlePointerMove"
    @pointerenter="handlePointerEnter"
    @pointerleave="handlePointerLeave"
    @pointerup="handlePointerUp"
    @pointercancel="handlePointerCancel"
  >
    <p v-if="errorMessage" class="pet-stage__error">{{ errorMessage }}</p>
  </div>
</template>

<style scoped>
.pet-stage {
  width: 100%;
  height: 100%;
  cursor: grab;
  touch-action: none;
}

.pet-stage:active {
  cursor: grabbing;
}

.pet-stage :deep(canvas) {
  display: block;
  width: 100%;
  height: 100%;
}

.pet-stage__error {
  position: absolute;
  inset: 8px;
  margin: 0;
  overflow: auto;
  border-radius: 8px;
  padding: 8px;
  color: #7f1d1d;
  background: rgb(254 226 226 / 90%);
  font-size: 12px;
}
</style>
