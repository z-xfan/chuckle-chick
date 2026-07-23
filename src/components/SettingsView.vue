<script setup lang="ts">
import { onMounted, ref } from "vue";

import PetPortrait from "@/components/PetPortrait.vue";
import {
  closeSettingsWindow,
  getPreferences,
  resetPetPosition,
  setPetAlwaysOnTop,
  setPetScale,
  setSpeechBubblesEnabled,
} from "@/platform/preferences";

const loading = ref(true);
const saving = ref(false);
const errorMessage = ref("");
const successMessage = ref("");
const alwaysOnTop = ref(true);
const speechBubblesEnabled = ref(true);
const scale = ref(1);
let scaleSaveTimer: number | undefined;

onMounted(async () => {
  try {
    const preferences = await getPreferences();
    alwaysOnTop.value = preferences.alwaysOnTop;
    speechBubblesEnabled.value = preferences.speechBubblesEnabled;
    scale.value = preferences.scale;
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    loading.value = false;
  }
});

async function saveAlwaysOnTop(): Promise<void> {
  saving.value = true;
  errorMessage.value = "";
  successMessage.value = "";
  try {
    await setPetAlwaysOnTop(alwaysOnTop.value);
  } catch (error) {
    alwaysOnTop.value = !alwaysOnTop.value;
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    saving.value = false;
  }
}

async function saveSpeechBubblesEnabled(): Promise<void> {
  saving.value = true;
  errorMessage.value = "";
  successMessage.value = "";
  try {
    await setSpeechBubblesEnabled(speechBubblesEnabled.value);
  } catch (error) {
    speechBubblesEnabled.value = !speechBubblesEnabled.value;
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    saving.value = false;
  }
}

async function saveScale(): Promise<void> {
  saving.value = true;
  errorMessage.value = "";
  successMessage.value = "";
  try {
    await setPetScale(scale.value);
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    saving.value = false;
  }
}

function queueScaleSave(): void {
  if (scaleSaveTimer !== undefined) window.clearTimeout(scaleSaveTimer);
  scaleSaveTimer = window.setTimeout(() => void saveScale(), 50);
}

async function restoreDefaultPosition(): Promise<void> {
  saving.value = true;
  errorMessage.value = "";
  successMessage.value = "";
  try {
    await resetPetPosition();
    successMessage.value = "宠物已回到主屏幕默认位置";
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <main class="settings">
    <header class="settings__header">
      <div>
        <p class="settings__eyebrow">ChuckleChick</p>
        <h1>宠物设置</h1>
      </div>
      <button type="button" class="settings__close" aria-label="关闭设置" @click="closeSettingsWindow">
        ×
      </button>
    </header>

    <section v-if="!loading" class="settings__panel">
      <label class="settings__row">
        <span>
          <strong>窗口置顶</strong>
          <small>让宠物保持在其他普通窗口上方</small>
        </span>
        <input v-model="alwaysOnTop" type="checkbox" :disabled="saving" @change="saveAlwaysOnTop" />
      </label>

      <label class="settings__row">
        <span>
          <strong>显示宠物气泡</strong>
          <small>允许黄鸡在互动时显示简短台词</small>
        </span>
        <input
          v-model="speechBubblesEnabled"
          type="checkbox"
          :disabled="saving"
          @change="saveSpeechBubblesEnabled"
        />
      </label>

      <label class="settings__scale">
        <span>
          <strong>宠物大小</strong>
          <small>{{ Math.round(scale * 100) }}%</small>
        </span>
        <input
          v-model.number="scale"
          type="range"
          min="0.4"
          max="2"
          step="0.05"
          @input="queueScaleSave"
        />
      </label>

      <div class="settings__pet">
        <span class="settings__pet-icon"><PetPortrait :size="38" /></span>
        <span><strong>黄鸡大笑</strong><small>当前宠物</small></span>
      </div>

      <button
        type="button"
        class="settings__reset-position"
        :disabled="saving"
        @click="restoreDefaultPosition"
      >
        恢复默认位置
      </button>
    </section>

    <p v-if="errorMessage" class="settings__error">{{ errorMessage }}</p>
    <p v-if="successMessage" class="settings__success">{{ successMessage }}</p>
    <p class="settings__note">更多互动设置将在后续独立需求中提供。</p>
  </main>
</template>

<style scoped>
.settings {
  min-height: 100%;
  padding: 24px;
  color: #382716;
  background: linear-gradient(145deg, #fffaf0, #fff1c2);
}

.settings__header,
.settings__row,
.settings__scale,
.settings__pet {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.settings__eyebrow {
  margin: 0 0 4px;
  color: #d97706;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

h1 {
  margin: 0;
  font-size: 24px;
}

.settings__close {
  width: 34px;
  height: 34px;
  border: 0;
  border-radius: 50%;
  color: #6b4423;
  background: rgb(255 255 255 / 65%);
  font-size: 24px;
  cursor: pointer;
}

.settings__panel {
  display: grid;
  gap: 12px;
  margin-top: 24px;
}

.settings__row,
.settings__scale,
.settings__pet {
  border: 1px solid rgb(217 119 6 / 15%);
  border-radius: 14px;
  padding: 16px;
  background: rgb(255 255 255 / 72%);
}

.settings__scale {
  display: grid;
}

.settings__scale > span {
  display: flex;
  justify-content: space-between;
}

.settings__scale input {
  width: 100%;
  accent-color: #f59e0b;
}

.settings strong,
.settings small {
  display: block;
}

.settings small {
  margin-top: 3px;
  color: #8a6848;
}

.settings__row input {
  width: 20px;
  height: 20px;
  accent-color: #f59e0b;
}

.settings__pet {
  justify-content: flex-start;
}

.settings__pet-icon {
  display: grid;
  width: 44px;
  height: 44px;
  place-items: center;
  border-radius: 13px;
  background: rgb(255 248 220 / 78%);
}

.settings__error {
  color: #b91c1c;
  font-size: 13px;
}

.settings__success {
  color: #15803d;
  font-size: 13px;
}

.settings__reset-position {
  border: 1px solid rgb(217 119 6 / 30%);
  border-radius: 12px;
  padding: 11px 16px;
  color: #7c4a03;
  background: rgb(255 255 255 / 72%);
  font-weight: 700;
  cursor: pointer;
}

.settings__reset-position:disabled {
  cursor: wait;
  opacity: 0.6;
}

.settings__note {
  color: #8a6848;
  font-size: 12px;
  text-align: center;
}
</style>
