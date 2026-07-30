<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import {
  checkServer,
  getServerOptions,
  setServerMonitoring,
  stopAllServerMonitoring,
  type ServerOption,
  type ServerStatus,
} from "@/platform/jx3";

defineEmits<{
  back: [];
  close: [];
}>();

const options = ref<ServerOption[]>([]);
const selectedName = ref("");
const currentStatus = ref<ServerStatus>();
const loading = ref(false);
const errorMessage = ref("");

const selected = computed(() =>
  options.value.find((server) => server.name === selectedName.value),
);
const monitored = computed(() => options.value.filter((server) => server.monitored));

onMounted(() => {
  void loadOptions();
});

async function loadOptions(): Promise<void> {
  loading.value = true;
  errorMessage.value = "";
  try {
    options.value = await getServerOptions();
    const firstServer = options.value[0];
    if (!selectedName.value && firstServer) {
      selectedName.value = firstServer.name;
    }
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    loading.value = false;
  }
}

async function querySelected(): Promise<void> {
  if (!selectedName.value || loading.value) return;
  loading.value = true;
  errorMessage.value = "";
  try {
    currentStatus.value = await checkServer(selectedName.value);
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    loading.value = false;
  }
}

async function toggleSelectedMonitoring(): Promise<void> {
  if (!selected.value || loading.value) return;
  loading.value = true;
  errorMessage.value = "";
  try {
    const names = await setServerMonitoring(selected.value.name, !selected.value.monitored);
    const monitoredNames = new Set(names);
    options.value = options.value.map((server) => ({
      ...server,
      monitored: monitoredNames.has(server.name),
    }));
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    loading.value = false;
  }
}

async function removeMonitoring(serverName: string): Promise<void> {
  if (loading.value) return;
  loading.value = true;
  errorMessage.value = "";
  try {
    const names = await setServerMonitoring(serverName, false);
    const monitoredNames = new Set(names);
    options.value = options.value.map((server) => ({
      ...server,
      monitored: monitoredNames.has(server.name),
    }));
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    loading.value = false;
  }
}

async function stopAll(): Promise<void> {
  if (loading.value) return;
  loading.value = true;
  errorMessage.value = "";
  try {
    await stopAllServerMonitoring();
    options.value = options.value.map((server) => ({ ...server, monitored: false }));
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    loading.value = false;
  }
}

function statusText(status?: ServerStatus["status"]): string {
  if (status === "open") return "已开服";
  if (status === "closed") return "维护中";
  return "未知";
}

function formatCheckedAt(value?: number): string {
  if (!value) return "";
  return new Date(value * 1000).toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}
</script>

<template>
  <section class="server-page" aria-label="开服状态">
    <header class="server-page__header">
      <button type="button" aria-label="返回快捷面板" @click="$emit('back')">←</button>
      <strong>开服状态</strong>
      <button type="button" aria-label="关闭快捷面板" @click="$emit('close')">×</button>
    </header>

    <div v-if="options.length === 0 && loading" class="server-page__state">
      正在读取官方服务器列表…
    </div>
    <div v-else-if="options.length === 0" class="server-page__state">
      <span>{{ errorMessage || "暂时无法读取服务器列表" }}</span>
      <button type="button" @click="loadOptions">重试</button>
    </div>
    <template v-else>
      <label class="server-page__select">
        <span>选择服务器</span>
        <select v-model="selectedName" :disabled="loading">
          <option v-for="server in options" :key="server.name" :value="server.name">
            {{ server.zone }} · {{ server.name }}
          </option>
        </select>
      </label>

      <div class="server-page__actions">
        <button type="button" :disabled="loading" @click="querySelected">
          {{ loading ? "查询中" : "单独查询" }}
        </button>
        <button type="button" :disabled="loading || !selected" @click="toggleSelectedMonitoring">
          {{ selected?.monitored ? "停止监听" : "监听开服" }}
        </button>
      </div>

      <div class="server-status" :class="`server-status--${currentStatus?.status ?? 'unknown'}`">
        <template v-if="currentStatus">
          <strong>{{ currentStatus.name }} · {{ statusText(currentStatus.status) }}</strong>
          <small>{{ currentStatus.zone }} · 查询于 {{ formatCheckedAt(currentStatus.checkedAt) }}</small>
        </template>
        <template v-else>
          <strong>尚未查询</strong>
          <small>选择服务器后可立即查询当前状态</small>
        </template>
      </div>

      <p v-if="errorMessage" class="server-page__error">{{ errorMessage }}</p>

      <div class="monitor-list">
        <header>
          <strong>正在监听（{{ monitored.length }}）</strong>
          <button v-if="monitored.length" type="button" :disabled="loading" @click="stopAll">
            全部停止
          </button>
        </header>
        <p v-if="monitored.length === 0">还没有选择服务器。监听期间会自适应查询并在开服时冒泡提醒。</p>
        <ul v-else>
          <li v-for="server in monitored" :key="server.name">
            <span>{{ server.name }}<small>{{ server.zone }}</small></span>
            <button type="button" :disabled="loading" @click="removeMonitoring(server.name)">
              停止
            </button>
          </li>
        </ul>
      </div>
    </template>
  </section>
</template>

<style scoped>
.server-page {
  display: grid;
  align-content: start;
  gap: 10px;
  width: calc(100% - 12px);
  height: calc(100% - 12px);
  margin: 6px;
  border: 1px solid rgb(217 119 6 / 22%);
  border-radius: 20px;
  padding: 12px;
  overflow: hidden;
  color: #382716;
  background: linear-gradient(145deg, rgb(255 250 240 / 98%), rgb(255 241 194 / 98%));
  box-shadow: 0 10px 30px rgb(83 51 18 / 20%);
}

.server-page__header,
.server-page__actions,
.monitor-list header,
.monitor-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.server-page__header strong {
  font-size: 16px;
}

.server-page button,
.server-page select {
  border: 1px solid rgb(217 119 6 / 20%);
  border-radius: 10px;
  color: #6b4423;
  background: rgb(255 255 255 / 78%);
  font: inherit;
}

.server-page button {
  cursor: pointer;
}

.server-page__header button {
  width: 30px;
  height: 30px;
  padding: 0;
  font-size: 19px;
}

.server-page__select {
  display: grid;
  gap: 5px;
  color: #8a6848;
  font-size: 11px;
}

.server-page__select select {
  width: 100%;
  padding: 8px;
}

.server-page__actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
}

.server-page__actions button {
  padding: 8px;
  font-weight: 700;
}

.server-status {
  display: grid;
  gap: 3px;
  border-radius: 12px;
  padding: 10px 12px;
  background: rgb(255 255 255 / 65%);
}

.server-status strong,
.server-status small {
  display: block;
}

.server-status strong {
  font-size: 13px;
}

.server-status small {
  color: #8a6848;
  font-size: 10px;
}

.server-status--open {
  box-shadow: inset 3px 0 #16a34a;
}

.server-status--closed {
  box-shadow: inset 3px 0 #dc2626;
}

.server-page__error {
  margin: 0;
  color: #b91c1c;
  font-size: 11px;
  text-align: center;
}

.server-page__state {
  display: grid;
  height: 300px;
  place-content: center;
  gap: 10px;
  color: #8a6848;
  font-size: 12px;
  text-align: center;
}

.server-page__state button {
  padding: 7px 12px;
}

.monitor-list {
  min-height: 0;
  overflow: hidden;
}

.monitor-list header {
  margin-bottom: 6px;
}

.monitor-list header strong {
  font-size: 12px;
}

.monitor-list header button,
.monitor-list li button {
  padding: 4px 8px;
  font-size: 10px;
}

.monitor-list > p {
  margin: 12px 6px;
  color: #8a6848;
  font-size: 11px;
  line-height: 1.5;
  text-align: center;
}

.monitor-list ul {
  max-height: 110px;
  margin: 0;
  padding: 0;
  overflow: auto;
  list-style: none;
}

.monitor-list li {
  border-radius: 9px;
  padding: 6px 8px;
  background: rgb(255 255 255 / 55%);
}

.monitor-list li + li {
  margin-top: 5px;
}

.monitor-list li span {
  font-size: 11px;
  font-weight: 700;
}

.monitor-list li small {
  margin-left: 6px;
  color: #9a7653;
  font-size: 9px;
  font-weight: 400;
}

button:disabled,
select:disabled {
  cursor: default;
  opacity: 0.5;
}

button:focus-visible,
select:focus-visible {
  outline: 3px solid rgb(245 158 11 / 45%);
  outline-offset: 2px;
}
</style>
