<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

import PetPortrait from "@/components/PetPortrait.vue";
import { SpeechBubbleQueue, type SpeechBubble } from "@/pet-core/SpeechBubbleQueue";
import {
  getAssistantPayload,
  hideAssistantWindow,
  listenAssistantPayload,
  requestPetInteraction,
  showSpeechBubble,
  type AssistantPayload,
} from "@/platform/assistant";
import {
  getPreferences,
  openSettingsWindow,
  setPetAlwaysOnTop,
} from "@/platform/preferences";

const mode = ref<"hidden" | "panel" | "bubble">("hidden");
const currentBubble = ref<SpeechBubble>();
const bubblePlacement = ref<"above" | "below">("above");
const alwaysOnTop = ref(true);
const busy = ref(false);
const errorMessage = ref("");
const interactionMessage = ref("");
const bubbleQueue = new SpeechBubbleQueue();
let bubbleTimer: number | undefined;
let blurTimer: number | undefined;
let stopListening: (() => void) | undefined;

onMounted(async () => {
  stopListening = await listenAssistantPayload(handlePayload);
  const initialPayload = await getAssistantPayload();
  if (initialPayload) handlePayload(initialPayload);
  window.addEventListener("blur", handleWindowBlur);
  window.addEventListener("keydown", handleKeyDown);
});

onBeforeUnmount(() => {
  stopListening?.();
  clearBubbleTimer();
  if (blurTimer !== undefined) window.clearTimeout(blurTimer);
  window.removeEventListener("blur", handleWindowBlur);
  window.removeEventListener("keydown", handleKeyDown);
});

function handlePayload(payload: AssistantPayload): void {
  if (payload.mode === "panel") {
    clearBubbleTimer();
    bubbleQueue.clear();
    currentBubble.value = undefined;
    mode.value = "panel";
    interactionMessage.value = "";
    errorMessage.value = "";
    void loadPanelPreferences();
    return;
  }

  const result = bubbleQueue.enqueue(payload.message, payload.durationMs);
  if (result === "empty" || result === "duplicate") return;
  bubblePlacement.value = payload.placement;
  mode.value = "bubble";
  if (!currentBubble.value) showCurrentBubble();
}

async function loadPanelPreferences(): Promise<void> {
  try {
    alwaysOnTop.value = (await getPreferences()).alwaysOnTop;
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

function showCurrentBubble(): void {
  const bubble = bubbleQueue.current;
  currentBubble.value = bubble;
  clearBubbleTimer();
  if (!bubble) {
    mode.value = "hidden";
    void hideAssistantWindow();
    return;
  }

  if ([...bubble.message].length > 60) {
    console.warn("宠物气泡文本超过 60 个字符，界面将省略超出内容");
  }
  bubbleTimer = window.setTimeout(finishCurrentBubble, bubble.durationMs);
}

function finishCurrentBubble(): void {
  bubbleQueue.finishCurrent();
  showCurrentBubble();
}

function clearBubbleTimer(): void {
  if (bubbleTimer !== undefined) window.clearTimeout(bubbleTimer);
  bubbleTimer = undefined;
}

async function closeAssistant(): Promise<void> {
  clearBubbleTimer();
  bubbleQueue.clear();
  currentBubble.value = undefined;
  mode.value = "hidden";
  await hideAssistantWindow();
}

async function greetPet(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  errorMessage.value = "";
  interactionMessage.value = "";
  try {
    await requestPetInteraction("waving");
    const shown = await showSpeechBubble("江湖路远，今天也一起开心闯荡吧！");
    if (!shown) interactionMessage.value = "黄鸡开心地向你挥了挥翅膀";
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    busy.value = false;
  }
}

async function playRandomInteraction(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  errorMessage.value = "";
  interactionMessage.value = "";
  try {
    await requestPetInteraction("random");
    interactionMessage.value = "黄鸡开始自由发挥";
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    busy.value = false;
  }
}

async function saveAlwaysOnTop(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  errorMessage.value = "";
  interactionMessage.value = "";
  try {
    await setPetAlwaysOnTop(alwaysOnTop.value);
    interactionMessage.value = alwaysOnTop.value ? "宠物已保持置顶" : "宠物已取消置顶";
  } catch (error) {
    alwaysOnTop.value = !alwaysOnTop.value;
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    busy.value = false;
  }
}

async function openSettings(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  errorMessage.value = "";
  try {
    await openSettingsWindow();
    mode.value = "hidden";
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    busy.value = false;
  }
}

function handleWindowBlur(): void {
  if (mode.value !== "panel") return;
  if (blurTimer !== undefined) window.clearTimeout(blurTimer);
  blurTimer = window.setTimeout(() => {
    if (mode.value === "panel") void closeAssistant();
  }, 120);
}

function handleKeyDown(event: KeyboardEvent): void {
  if (event.key !== "Escape" || mode.value === "hidden") return;
  event.preventDefault();
  void closeAssistant();
}
</script>

<template>
  <main class="assistant-root" :class="`assistant-root--${mode}`">
    <button
      v-if="mode === 'bubble' && currentBubble"
      type="button"
      class="speech-bubble"
      :class="`speech-bubble--${bubblePlacement}`"
      aria-label="关闭宠物气泡"
      @click="closeAssistant"
    >
      <span v-text="currentBubble.message"></span>
      <i aria-hidden="true"></i>
    </button>

    <section v-else-if="mode === 'panel'" class="quick-panel" aria-label="宠物快捷功能">
      <header class="quick-panel__header">
        <span class="quick-panel__avatar">
          <PetPortrait :size="34" />
        </span>
        <span class="quick-panel__identity">
          <strong>黄鸡大笑</strong>
          <small>今天也在陪你闯荡江湖</small>
        </span>
        <button
          type="button"
          class="quick-panel__close"
          aria-label="关闭快捷功能面板"
          @click="closeAssistant"
        >
          ×
        </button>
      </header>

      <div class="quick-panel__actions" aria-label="宠物互动">
        <button type="button" :disabled="busy" @click="greetPet">
          <span aria-hidden="true">👋</span>
          <strong>打个招呼</strong>
        </button>
        <button type="button" :disabled="busy" @click="playRandomInteraction">
          <span aria-hidden="true">✨</span>
          <strong>随机动作</strong>
        </button>
      </div>

      <label class="quick-panel__setting">
        <span>
          <strong>窗口置顶</strong>
          <small>让宠物保持在普通窗口上方</small>
        </span>
        <input
          v-model="alwaysOnTop"
          type="checkbox"
          :disabled="busy"
          @change="saveAlwaysOnTop"
        />
      </label>

      <p v-if="errorMessage" class="quick-panel__message quick-panel__message--error">
        {{ errorMessage }}
      </p>
      <p v-else-if="interactionMessage" class="quick-panel__message">
        {{ interactionMessage }}
      </p>

      <button type="button" class="quick-panel__settings" :disabled="busy" @click="openSettings">
        打开宠物设置
      </button>
    </section>
  </main>
</template>

<style scoped>
.assistant-root {
  width: 100%;
  height: 100%;
  overflow: hidden;
  color: #382716;
  background: transparent;
  font-family: system-ui, sans-serif;
}

.assistant-root--hidden {
  pointer-events: none;
}

.speech-bubble {
  position: relative;
  display: grid;
  width: calc(100% - 16px);
  height: calc(100% - 22px);
  margin: 8px;
  place-items: center;
  border: 1px solid rgb(217 119 6 / 30%);
  border-radius: 20px;
  padding: 16px 20px;
  color: #4b321b;
  background: rgb(255 250 235 / 98%);
  box-shadow: 0 8px 24px rgb(83 51 18 / 18%);
  font: inherit;
  font-size: 14px;
  line-height: 1.55;
  text-align: left;
  cursor: pointer;
}

.speech-bubble span {
  display: -webkit-box;
  width: 100%;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}

.speech-bubble i {
  position: absolute;
  bottom: -9px;
  left: calc(50% - 9px);
  width: 18px;
  height: 18px;
  border-right: 1px solid rgb(217 119 6 / 30%);
  border-bottom: 1px solid rgb(217 119 6 / 30%);
  background: #fffaf0;
  transform: rotate(45deg);
}

.speech-bubble--below {
  margin-top: 14px;
  margin-bottom: 2px;
}

.speech-bubble--below i {
  top: -9px;
  bottom: auto;
  border: 0;
  border-top: 1px solid rgb(217 119 6 / 30%);
  border-left: 1px solid rgb(217 119 6 / 30%);
}

.quick-panel {
  display: grid;
  gap: 12px;
  width: calc(100% - 12px);
  height: calc(100% - 12px);
  margin: 6px;
  border: 1px solid rgb(217 119 6 / 22%);
  border-radius: 20px;
  padding: 16px;
  overflow: hidden;
  background: linear-gradient(145deg, rgb(255 250 240 / 98%), rgb(255 241 194 / 98%));
  box-shadow: 0 10px 30px rgb(83 51 18 / 20%);
}

.quick-panel__header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.quick-panel__avatar {
  display: grid;
  width: 40px;
  height: 40px;
  flex: 0 0 auto;
  place-items: center;
  border-radius: 13px;
  background: rgb(255 255 255 / 72%);
}

.quick-panel__identity {
  min-width: 0;
  flex: 1;
}

.quick-panel__identity strong,
.quick-panel__identity small,
.quick-panel__setting strong,
.quick-panel__setting small {
  display: block;
}

.quick-panel__identity strong {
  font-size: 16px;
}

.quick-panel__identity small,
.quick-panel__setting small {
  margin-top: 2px;
  color: #8a6848;
  font-size: 11px;
}

.quick-panel__close {
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 50%;
  color: #6b4423;
  background: rgb(255 255 255 / 65%);
  font-size: 21px;
  line-height: 1;
  cursor: pointer;
}

.quick-panel__actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.quick-panel__actions button,
.quick-panel__settings {
  border: 1px solid rgb(217 119 6 / 18%);
  border-radius: 13px;
  color: #6b4423;
  background: rgb(255 255 255 / 76%);
  cursor: pointer;
}

.quick-panel__actions button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  min-height: 48px;
  padding: 10px;
}

.quick-panel__actions button span {
  font-size: 18px;
}

.quick-panel__setting {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border-radius: 13px;
  padding: 10px 12px;
  background: rgb(255 255 255 / 58%);
}

.quick-panel__setting input {
  width: 20px;
  height: 20px;
  accent-color: #f59e0b;
}

.quick-panel__message {
  min-height: 18px;
  margin: -4px 2px;
  color: #8a5a24;
  font-size: 12px;
  text-align: center;
}

.quick-panel__message--error {
  color: #b91c1c;
}

.quick-panel__settings {
  min-height: 38px;
  padding: 9px 14px;
  font-weight: 700;
}

button:focus-visible,
input:focus-visible {
  outline: 3px solid rgb(245 158 11 / 45%);
  outline-offset: 2px;
}

button:disabled,
input:disabled {
  cursor: wait;
  opacity: 0.6;
}

@media (prefers-reduced-motion: no-preference) {
  .speech-bubble,
  .quick-panel {
    animation: assistant-enter 120ms ease-out;
  }
}

@keyframes assistant-enter {
  from {
    opacity: 0;
    transform: translateY(4px) scale(0.98);
  }

  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
</style>
