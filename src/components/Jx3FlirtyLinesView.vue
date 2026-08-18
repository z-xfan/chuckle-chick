<script lang="ts">
import type { FlirtyLineKind, FlirtyLineView } from "@/platform/jx3";

const sessionLines: Partial<Record<FlirtyLineKind, FlirtyLineView>> = {};
const cooldownUntil: Record<FlirtyLineKind, number> = {
  random: 0,
  devoted: 0,
  scumbag: 0,
};
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

import { writePlainText } from "@/platform/clipboard";
import { fetchFlirtyLine } from "@/platform/jx3";

defineEmits<{
  back: [];
  close: [];
}>();

const TYPES: Array<{ kind: FlirtyLineKind; label: string; marker: string }> = [
  { kind: "random", label: "随机骚话", marker: "随" },
  { kind: "devoted", label: "舔狗骚话", marker: "舔" },
  { kind: "scumbag", label: "渣男语录", marker: "渣" },
];
const REQUEST_COOLDOWN_MS = 1_000;
const COPY_FEEDBACK_MS = 1_600;

const selectedKind = ref<FlirtyLineKind>("random");
const lines = ref<Partial<Record<FlirtyLineKind, FlirtyLineView>>>({
  ...sessionLines,
});
const loadingKinds = ref<Record<FlirtyLineKind, boolean>>({
  random: false,
  devoted: false,
  scumbag: false,
});
const requestError = ref("");
const copyError = ref("");
const copied = ref(false);
const cooldownActive = ref(false);
let cooldownTimer: number | undefined;
let copyFeedbackTimer: number | undefined;

const currentLine = computed(() => lines.value[selectedKind.value]);
const loading = computed(() => loadingKinds.value[selectedKind.value]);
const selectedLabel = computed(
  () => TYPES.find((item) => item.kind === selectedKind.value)?.label ?? "骚话",
);

onMounted(() => {
  armCooldownTimer();
  if (!currentLine.value) void loadLine(selectedKind.value);
});

onBeforeUnmount(() => {
  if (cooldownTimer !== undefined) window.clearTimeout(cooldownTimer);
  if (copyFeedbackTimer !== undefined) window.clearTimeout(copyFeedbackTimer);
});

function selectKind(kind: FlirtyLineKind): void {
  if (kind === selectedKind.value) return;
  selectedKind.value = kind;
  requestError.value = "";
  copyError.value = "";
  copied.value = false;
  armCooldownTimer();
  if (!currentLine.value) void loadLine(kind);
}

async function loadLine(kind: FlirtyLineKind): Promise<void> {
  if (loadingKinds.value[kind]) return;
  const remaining = cooldownUntil[kind] - Date.now();
  if (remaining > 0) {
    armCooldownTimer();
    return;
  }

  loadingKinds.value[kind] = true;
  cooldownUntil[kind] = Date.now() + REQUEST_COOLDOWN_MS;
  if (selectedKind.value === kind) {
    requestError.value = "";
    copyError.value = "";
    copied.value = false;
    armCooldownTimer();
  }

  try {
    const line = await fetchFlirtyLine(kind);
    sessionLines[kind] = line;
    lines.value = { ...lines.value, [kind]: line };
  } catch (error) {
    if (selectedKind.value === kind) {
      requestError.value = error instanceof Error ? error.message : String(error);
    }
  } finally {
    loadingKinds.value[kind] = false;
  }
}

async function copyCurrentLine(): Promise<void> {
  const line = currentLine.value;
  if (!line || loading.value) return;
  copyError.value = "";
  try {
    await writePlainText(line.text);
    copied.value = true;
    if (copyFeedbackTimer !== undefined) window.clearTimeout(copyFeedbackTimer);
    copyFeedbackTimer = window.setTimeout(() => {
      copied.value = false;
    }, COPY_FEEDBACK_MS);
  } catch (error) {
    copied.value = false;
    copyError.value = `复制失败：${error instanceof Error ? error.message : String(error)}`;
  }
}

function armCooldownTimer(): void {
  if (cooldownTimer !== undefined) window.clearTimeout(cooldownTimer);
  const remaining = cooldownUntil[selectedKind.value] - Date.now();
  cooldownActive.value = remaining > 0;
  if (remaining > 0) {
    cooldownTimer = window.setTimeout(() => {
      cooldownActive.value = false;
    }, remaining);
  }
}
</script>

<template>
  <section class="flirty-page" aria-label="骚话文案">
    <header class="flirty-page__header">
      <button type="button" aria-label="返回快捷面板" @click="$emit('back')">←</button>
      <strong>骚话</strong>
      <button type="button" aria-label="关闭快捷面板" @click="$emit('close')">×</button>
    </header>

    <div class="flirty-tabs" aria-label="选择骚话类型">
      <button
        v-for="item in TYPES"
        :key="item.kind"
        type="button"
        :class="{ 'flirty-tabs__button--active': selectedKind === item.kind }"
        :aria-pressed="selectedKind === item.kind"
        @click="selectKind(item.kind)"
      >
        <span aria-hidden="true">{{ item.marker }}</span>
        <strong>{{ item.label }}</strong>
      </button>
    </div>

    <div class="flirty-page__content">
      <div v-if="!currentLine && loading" class="flirty-page__state">
        <span class="flirty-page__loading" aria-hidden="true">…</span>
        正在寻找一句合适的话
      </div>

      <div v-else-if="!currentLine" class="flirty-page__state">
        <span>{{ requestError || "暂时没有可用内容" }}</span>
        <button
          type="button"
          :disabled="loading || cooldownActive"
          @click="loadLine(selectedKind)"
        >
          重试
        </button>
      </div>

      <div
        v-else
        class="flirty-card"
        role="button"
        tabindex="0"
        :aria-label="`复制当前${selectedLabel}`"
        @click="copyCurrentLine"
        @keydown.enter.prevent="copyCurrentLine"
        @keydown.space.prevent="copyCurrentLine"
      >
        <span class="flirty-card__quote" aria-hidden="true">“</span>
        <p v-text="currentLine.text"></p>
        <small>点击整段复制</small>
      </div>

      <p
        v-if="copyError"
        class="flirty-page__notice flirty-page__notice--error"
        role="status"
      >
        {{ copyError }}
      </p>
      <p v-else-if="copied" class="flirty-page__notice flirty-page__notice--success" role="status">
        已复制到剪贴板
      </p>
      <p
        v-else-if="currentLine && requestError"
        class="flirty-page__notice flirty-page__notice--error"
        role="status"
      >
        {{ requestError }}
      </p>
    </div>

    <div class="flirty-page__actions">
      <button
        type="button"
        class="flirty-page__copy"
        :disabled="!currentLine || loading"
        @click="copyCurrentLine"
      >
        {{ copied ? "✓ 已复制" : "复制" }}
      </button>
      <button
        type="button"
        class="flirty-page__refresh"
        :disabled="loading || cooldownActive"
        @click="loadLine(selectedKind)"
      >
        {{ loading ? "寻找中…" : "换一句" }}
      </button>
    </div>

    <footer class="flirty-page__footer">
      <strong>{{ currentLine?.source ?? "JX3API（第三方）" }}</strong>
      <span>随机文案可能包含粗口或冒犯表达</span>
    </footer>
  </section>
</template>

<style scoped>
.flirty-page {
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr) auto auto;
  gap: 10px;
  width: calc(100% - 12px);
  height: calc(100% - 12px);
  margin: 6px;
  border: 1px solid rgb(217 119 6 / 22%);
  border-radius: 20px;
  padding: 14px;
  overflow: hidden;
  color: #382716;
  background:
    radial-gradient(circle at 84% 8%, rgb(251 191 36 / 24%), transparent 32%),
    linear-gradient(145deg, rgb(255 250 240 / 98%), rgb(255 241 194 / 98%));
  box-shadow: 0 10px 30px rgb(83 51 18 / 20%);
}

.flirty-page__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.flirty-page__header strong {
  font-size: 16px;
}

.flirty-page button {
  border: 1px solid rgb(217 119 6 / 20%);
  border-radius: 10px;
  color: #6b4423;
  background: rgb(255 255 255 / 78%);
  font: inherit;
  cursor: pointer;
}

.flirty-page__header button {
  width: 30px;
  height: 30px;
  padding: 0;
  font-size: 19px;
}

.flirty-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 7px;
  padding: 4px;
  border-radius: 14px;
  background: rgb(255 255 255 / 42%);
}

.flirty-tabs button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 7px 4px;
  border-color: transparent;
  background: transparent;
}

.flirty-tabs button span {
  display: grid;
  width: 22px;
  height: 22px;
  border-radius: 8px;
  place-items: center;
  color: #fff;
  background: #a07145;
  font-size: 10px;
  font-weight: 800;
}

.flirty-tabs button strong {
  overflow: hidden;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.flirty-tabs .flirty-tabs__button--active {
  border-color: rgb(217 119 6 / 35%);
  color: #92400e;
  background: #fde68a;
  box-shadow: 0 2px 8px rgb(146 64 14 / 10%);
}

.flirty-tabs .flirty-tabs__button--active span {
  background: #d97706;
}

.flirty-page__content {
  position: relative;
  min-height: 0;
}

.flirty-page__state {
  display: grid;
  height: 100%;
  place-content: center;
  gap: 10px;
  color: #8a6848;
  font-size: 12px;
  text-align: center;
}

.flirty-page__state button {
  justify-self: center;
  padding: 7px 14px;
}

.flirty-page__loading {
  display: grid;
  width: 36px;
  height: 36px;
  margin: 0 auto;
  border-radius: 50%;
  place-items: center;
  color: #fff;
  background: #d97706;
  font-size: 22px;
  line-height: 1;
}

.flirty-card {
  position: relative;
  display: grid;
  grid-template-rows: minmax(0, 1fr) auto;
  gap: 8px;
  height: 100%;
  min-height: 0;
  border: 1px solid rgb(217 119 6 / 22%);
  border-radius: 17px;
  padding: 18px 16px 12px;
  overflow: hidden;
  background: rgb(255 255 255 / 70%);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 80%);
  cursor: copy;
}

.flirty-card__quote {
  position: absolute;
  top: 3px;
  left: 10px;
  color: rgb(217 119 6 / 30%);
  font-family: Georgia, serif;
  font-size: 36px;
  line-height: 1;
}

.flirty-card p {
  min-height: 0;
  margin: 0;
  padding: 4px 2px;
  overflow-y: auto;
  color: #4b321b;
  font-size: 14px;
  font-weight: 600;
  line-height: 1.75;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.flirty-card small {
  color: #a17a54;
  font-size: 10px;
  text-align: right;
}

.flirty-card:focus-visible {
  outline: 3px solid rgb(245 158 11 / 45%);
  outline-offset: 2px;
}

.flirty-page__notice {
  position: absolute;
  right: 10px;
  bottom: 8px;
  left: 10px;
  margin: 0;
  border-radius: 9px;
  padding: 6px 8px;
  font-size: 10px;
  text-align: center;
  backdrop-filter: blur(4px);
}

.flirty-page__notice--success {
  color: #166534;
  background: rgb(220 252 231 / 92%);
}

.flirty-page__notice--error {
  color: #991b1b;
  background: rgb(254 226 226 / 94%);
}

.flirty-page__actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.flirty-page__actions button {
  min-height: 38px;
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 800;
}

.flirty-page__actions .flirty-page__copy {
  color: #fff;
  border-color: #d97706;
  background: #d97706;
}

.flirty-page__actions .flirty-page__refresh {
  background: rgb(255 255 255 / 88%);
}

.flirty-page__footer {
  display: grid;
  gap: 2px;
  color: #9a7650;
  font-size: 9px;
  text-align: center;
}

.flirty-page__footer strong {
  color: #7c5837;
}

.flirty-page button:disabled {
  cursor: default;
  opacity: 0.5;
}

.flirty-page button:focus-visible {
  outline: 3px solid rgb(245 158 11 / 45%);
  outline-offset: 2px;
}
</style>
