<script lang="ts">
import type { DecisionKind, DecisionView } from "@/platform/jx3";

const sessionDecisions: Partial<Record<DecisionKind, DecisionView>> = {};
const cooldownUntil: Record<DecisionKind, number> = {
  answer: 0,
  eat: 0,
  drink: 0,
};
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

import { writePlainText } from "@/platform/clipboard";
import { fetchDecision } from "@/platform/jx3";

defineEmits<{
  back: [];
  close: [];
}>();

const TYPES: Array<{ kind: DecisionKind; label: string; marker: string }> = [
  { kind: "answer", label: "答案之书", marker: "答" },
  { kind: "eat", label: "吃什么", marker: "吃" },
  { kind: "drink", label: "喝什么", marker: "喝" },
];
const REQUEST_COOLDOWN_MS = 1_000;
const COPY_FEEDBACK_MS = 1_600;

const selectedKind = ref<DecisionKind>("answer");
const decisions = ref<Partial<Record<DecisionKind, DecisionView>>>({
  ...sessionDecisions,
});
const loadingKinds = ref<Record<DecisionKind, boolean>>({
  answer: false,
  eat: false,
  drink: false,
});
const requestError = ref("");
const copyError = ref("");
const copied = ref(false);
const cooldownActive = ref(false);
let cooldownTimer: number | undefined;
let copyFeedbackTimer: number | undefined;

const currentDecision = computed(() => decisions.value[selectedKind.value]);
const loading = computed(() => loadingKinds.value[selectedKind.value]);
const selectedLabel = computed(
  () => TYPES.find((item) => item.kind === selectedKind.value)?.label ?? "小决定",
);
const actionLabel = computed(() => {
  if (loading.value) return "寻找中…";
  if (selectedKind.value === "answer") {
    return currentDecision.value ? "再翻一次" : "翻开答案";
  }
  return currentDecision.value ? "换一组" : "帮我选";
});

onMounted(armCooldownTimer);

onBeforeUnmount(() => {
  if (cooldownTimer !== undefined) window.clearTimeout(cooldownTimer);
  if (copyFeedbackTimer !== undefined) window.clearTimeout(copyFeedbackTimer);
});

function selectKind(kind: DecisionKind): void {
  if (kind === selectedKind.value) return;
  selectedKind.value = kind;
  requestError.value = "";
  copyError.value = "";
  copied.value = false;
  armCooldownTimer();
  if (kind !== "answer" && !currentDecision.value) void loadDecision(kind);
}

async function loadDecision(kind: DecisionKind): Promise<void> {
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
    const decision = await fetchDecision(kind);
    sessionDecisions[kind] = decision;
    decisions.value = { ...decisions.value, [kind]: decision };
  } catch (error) {
    if (selectedKind.value === kind) {
      requestError.value = error instanceof Error ? error.message : String(error);
    }
  } finally {
    loadingKinds.value[kind] = false;
  }
}

async function copyCurrentDecision(): Promise<void> {
  const decision = currentDecision.value;
  if (!decision || loading.value) return;
  copyError.value = "";
  try {
    await writePlainText(decision.items.join("\n"));
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
  <section class="decision-page" aria-label="小决定">
    <header class="decision-page__header">
      <button type="button" aria-label="返回快捷面板" @click="$emit('back')">←</button>
      <strong>小决定</strong>
      <button type="button" aria-label="关闭快捷面板" @click="$emit('close')">×</button>
    </header>

    <div class="decision-tabs" aria-label="选择小决定类型">
      <button
        v-for="item in TYPES"
        :key="item.kind"
        type="button"
        :class="{ 'decision-tabs__button--active': selectedKind === item.kind }"
        :aria-pressed="selectedKind === item.kind"
        @click="selectKind(item.kind)"
      >
        <span aria-hidden="true">{{ item.marker }}</span>
        <strong>{{ item.label }}</strong>
      </button>
    </div>

    <div class="decision-page__content">
      <div v-if="!currentDecision && loading" class="decision-page__state">
        <span class="decision-page__loading" aria-hidden="true">…</span>
        正在寻找{{ selectedLabel }}
      </div>

      <div v-else-if="!currentDecision" class="decision-page__state">
        <span v-if="selectedKind === 'answer' && !requestError" class="decision-page__question">
          先在心里想好一个问题<br />准备好后再翻开答案
        </span>
        <span v-else>{{ requestError || `暂时没有${selectedLabel}` }}</span>
        <button
          type="button"
          :disabled="loading || cooldownActive"
          @click="loadDecision(selectedKind)"
        >
          {{ selectedKind === "answer" ? "翻开答案" : "重试" }}
        </button>
      </div>

      <div
        v-else
        class="decision-card"
        role="button"
        tabindex="0"
        :aria-label="`复制当前${selectedLabel}结果`"
        @click="copyCurrentDecision"
        @keydown.enter.prevent="copyCurrentDecision"
        @keydown.space.prevent="copyCurrentDecision"
      >
        <template v-if="selectedKind === 'answer'">
          <span class="decision-card__eyebrow">书中的答案</span>
          <p class="decision-card__answer" v-text="currentDecision.items[0]"></p>
          <p class="decision-card__hearten" v-text="currentDecision.items[1]"></p>
        </template>
        <template v-else>
          <span class="decision-card__eyebrow">这次可以选</span>
          <ol class="decision-card__choices">
            <li
              v-for="(item, index) in currentDecision.items"
              :key="`${index}-${item}`"
              v-text="item"
            ></li>
          </ol>
        </template>
        <small>点击整张卡片复制全部结果</small>
      </div>

      <p
        v-if="copyError"
        class="decision-page__notice decision-page__notice--error"
        role="status"
      >
        {{ copyError }}
      </p>
      <p
        v-else-if="copied"
        class="decision-page__notice decision-page__notice--success"
        role="status"
      >
        已复制到剪贴板
      </p>
      <p
        v-else-if="currentDecision && requestError"
        class="decision-page__notice decision-page__notice--error"
        role="status"
      >
        {{ requestError }}
      </p>
    </div>

    <div class="decision-page__actions">
      <button
        type="button"
        class="decision-page__copy"
        :disabled="!currentDecision || loading"
        @click="copyCurrentDecision"
      >
        {{ copied ? "✓ 已复制" : "复制全部" }}
      </button>
      <button
        type="button"
        class="decision-page__refresh"
        :disabled="loading || cooldownActive"
        @click="loadDecision(selectedKind)"
      >
        {{ actionLabel }}
      </button>
    </div>

    <footer class="decision-page__footer">
      <strong>{{ currentDecision?.source ?? "JX3API（第三方）" }}</strong>
      <span>随机结果仅供轻松参考</span>
    </footer>
  </section>
</template>

<style scoped>
.decision-page {
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr) auto auto;
  gap: 10px;
  width: calc(100% - 12px);
  height: calc(100% - 12px);
  margin: 6px;
  border: 1px solid rgb(20 184 166 / 24%);
  border-radius: 20px;
  padding: 14px;
  overflow: hidden;
  color: #173f3a;
  background:
    radial-gradient(circle at 84% 8%, rgb(45 212 191 / 22%), transparent 32%),
    linear-gradient(145deg, rgb(240 253 250 / 98%), rgb(204 251 241 / 98%));
  box-shadow: 0 10px 30px rgb(15 118 110 / 18%);
}

.decision-page__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.decision-page__header strong {
  font-size: 16px;
}

.decision-page button {
  border: 1px solid rgb(13 148 136 / 22%);
  border-radius: 10px;
  color: #115e59;
  background: rgb(255 255 255 / 80%);
  font: inherit;
  cursor: pointer;
}

.decision-page__header button {
  width: 30px;
  height: 30px;
  padding: 0;
  font-size: 19px;
}

.decision-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 6px;
  padding: 4px;
  border-radius: 14px;
  background: rgb(255 255 255 / 45%);
}

.decision-tabs button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  min-width: 0;
  padding: 7px 4px;
  border-color: transparent;
  background: transparent;
}

.decision-tabs button span {
  display: grid;
  width: 22px;
  height: 22px;
  flex: 0 0 auto;
  border-radius: 8px;
  place-items: center;
  color: #fff;
  background: #5f8d87;
  font-size: 10px;
  font-weight: 800;
}

.decision-tabs button strong {
  overflow: hidden;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.decision-tabs .decision-tabs__button--active {
  border-color: rgb(13 148 136 / 34%);
  color: #115e59;
  background: #99f6e4;
  box-shadow: 0 2px 8px rgb(15 118 110 / 10%);
}

.decision-tabs .decision-tabs__button--active span {
  background: #0d9488;
}

.decision-page__content {
  position: relative;
  min-height: 0;
}

.decision-page__state {
  display: grid;
  height: 100%;
  place-content: center;
  gap: 14px;
  color: #4f7772;
  font-size: 12px;
  line-height: 1.7;
  text-align: center;
}

.decision-page__state button {
  justify-self: center;
  padding: 8px 16px;
  font-weight: 700;
}

.decision-page__question {
  font-size: 13px;
}

.decision-page__loading {
  display: grid;
  width: 36px;
  height: 36px;
  margin: 0 auto;
  border-radius: 50%;
  place-items: center;
  color: #fff;
  background: #0d9488;
  font-size: 22px;
  line-height: 1;
}

.decision-card {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  gap: 8px;
  height: 100%;
  min-height: 0;
  border: 1px solid rgb(13 148 136 / 22%);
  border-radius: 17px;
  padding: 14px 16px 11px;
  overflow: hidden;
  background: rgb(255 255 255 / 72%);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 80%);
  cursor: copy;
}

.decision-card__eyebrow {
  color: #0f766e;
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-align: center;
}

.decision-card__answer {
  align-self: center;
  margin: 0;
  overflow-y: auto;
  color: #134e4a;
  font-size: 19px;
  font-weight: 800;
  line-height: 1.55;
  text-align: center;
  overflow-wrap: anywhere;
}

.decision-card__hearten {
  margin: 0;
  color: #4f7772;
  font-size: 11px;
  line-height: 1.5;
  text-align: center;
  overflow-wrap: anywhere;
}

.decision-card__choices {
  min-height: 0;
  margin: 0;
  padding: 4px 4px 4px 28px;
  overflow-y: auto;
  color: #134e4a;
  font-size: 14px;
  font-weight: 700;
  line-height: 1.75;
  overflow-wrap: anywhere;
}

.decision-card small {
  color: #62908a;
  font-size: 10px;
  text-align: right;
}

.decision-card:focus-visible {
  outline: 3px solid rgb(20 184 166 / 38%);
  outline-offset: 2px;
}

.decision-page__notice {
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

.decision-page__notice--success {
  color: #166534;
  background: rgb(220 252 231 / 92%);
}

.decision-page__notice--error {
  color: #991b1b;
  background: rgb(254 226 226 / 94%);
}

.decision-page__actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.decision-page__actions button {
  min-height: 38px;
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 800;
}

.decision-page__actions .decision-page__copy {
  color: #fff;
  border-color: #0d9488;
  background: #0d9488;
}

.decision-page__footer {
  display: grid;
  gap: 2px;
  color: #62908a;
  font-size: 9px;
  text-align: center;
}

.decision-page__footer strong {
  color: #376d67;
}

.decision-page button:disabled {
  cursor: default;
  opacity: 0.5;
}

.decision-page button:focus-visible {
  outline: 3px solid rgb(20 184 166 / 38%);
  outline-offset: 2px;
}
</style>
