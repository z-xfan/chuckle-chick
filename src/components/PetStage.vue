<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

import { loadPetAsset } from "@/pet-assets/loadPetAsset";
import type { PetAtlas } from "@/pet-assets/types";
import {
  resolveDragDirection,
  resolveLookDirection,
  type DragDirection,
} from "@/pet-core/DirectionalInteraction";
import { RandomActionScheduler } from "@/pet-core/RandomActionScheduler";
import { PixiPetRenderer } from "@/pet-renderer/PixiPetRenderer";
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
let dragButtonCheckInFlight = false;
let supportsDragButtonPolling: boolean | undefined;
let cursorCheckInFlight = false;
let isDragging = false;
let isHovering = false;
let isLooking = false;
let lastWindowX: number | undefined;
let currentDragDirection: DragDirection | undefined;
let currentLookFrame = "";

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
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
});

onBeforeUnmount(() => {
  scheduler?.stop();
  stopObservingMovement?.();
  if (cursorTimer !== undefined) window.clearInterval(cursorTimer);
  if (dragEndTimer !== undefined) window.clearTimeout(dragEndTimer);
  if (dragButtonTimer !== undefined) window.clearInterval(dragButtonTimer);
  renderer?.destroy();
});

async function handlePointerDown(): Promise<void> {
  if (isDragging) return;
  isDragging = true;
  lastWindowX = undefined;
  isLooking = false;
  currentLookFrame = "";
  currentDragDirection = undefined;
  supportsDragButtonPolling = undefined;
  scheduler?.pause();
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
  if (isHovering) {
    await renderer?.play("jumping");
  } else {
    await renderer?.play("idle");
    scheduler?.resume();
  }
}

function handlePointerEnter(): void {
  isHovering = true;
  if (isDragging) return;
  isLooking = false;
  currentLookFrame = "";
  scheduler?.pause();
  void renderer?.play("jumping");
}

function handlePointerLeave(): void {
  isHovering = false;
  if (isDragging) return;
  isLooking = false;
  currentLookFrame = "";
  scheduler?.resume();
}

async function updateCursorLook(): Promise<void> {
  if (isDragging || isHovering || cursorCheckInFlight || lookDirections.length === 0) return;
  cursorCheckInFlight = true;
  try {
    const snapshot = await getPointerSnapshot();
    if (isDragging || isHovering) return;
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
</script>

<template>
  <div
    ref="host"
    class="pet-stage"
    role="img"
    aria-label="黄鸡大笑桌面宠物"
    @pointerdown.left="handlePointerDown"
    @pointerenter="handlePointerEnter"
    @pointerleave="handlePointerLeave"
    @pointerup="finishDragging"
    @pointercancel="finishDragging"
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
