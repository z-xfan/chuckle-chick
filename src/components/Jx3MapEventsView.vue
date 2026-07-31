<script setup lang="ts">
import { onMounted, ref } from "vue";

import {
  fetchMapEvents,
  getCachedMapEvents,
  type MapEventCategory,
  type MapEventView,
} from "@/platform/jx3";

defineEmits<{
  back: [];
  close: [];
}>();

const CATEGORIES: Array<{ name: MapEventCategory; marker: string }> = [
  { name: "楚天社", marker: "楚" },
  { name: "云从社", marker: "云" },
  { name: "披风会", marker: "披" },
];
const CATEGORY_STORAGE_KEY = "chuckle-chick:jx3-map-event-category";

const selectedCategory = ref<MapEventCategory>(readSavedCategory());
const view = ref<MapEventView>();
const loading = ref(false);
const errorMessage = ref("");
const staleMessage = ref("");
let requestSerial = 0;

onMounted(() => {
  void loadCategory();
});

function readSavedCategory(): MapEventCategory {
  try {
    const saved = window.localStorage.getItem(CATEGORY_STORAGE_KEY);
    if (CATEGORIES.some((category) => category.name === saved)) {
      return saved as MapEventCategory;
    }
  } catch {
    // Local storage can be unavailable in hardened WebViews; use the confirmed default.
  }
  return "楚天社";
}

function saveCategory(category: MapEventCategory): void {
  try {
    window.localStorage.setItem(CATEGORY_STORAGE_KEY, category);
  } catch {
    // Selection persistence is best-effort and must not block querying.
  }
}

function selectCategory(category: MapEventCategory): void {
  if (category === selectedCategory.value) return;
  selectedCategory.value = category;
  saveCategory(category);
  view.value = undefined;
  errorMessage.value = "";
  staleMessage.value = "";
  void loadCategory();
}

async function loadCategory(force = false): Promise<void> {
  const category = selectedCategory.value;
  const request = ++requestSerial;
  loading.value = true;
  errorMessage.value = "";
  staleMessage.value = "";
  let cached: MapEventView | null = null;
  try {
    if (!force) {
      cached = await getCachedMapEvents(category);
      if (request !== requestSerial) return;
      if (cached) {
        view.value = cached;
        if (!cached.stale) return;
        staleMessage.value = "正在更新上次数据…";
      }
    }
    const refreshed = await fetchMapEvents(category, force);
    if (request !== requestSerial) return;
    view.value = refreshed;
    staleMessage.value = refreshed.stale ? "当前显示上次数据" : "";
  } catch (error) {
    if (request !== requestSerial) return;
    errorMessage.value = error instanceof Error ? error.message : String(error);
    if (cached || view.value) staleMessage.value = "刷新失败，当前显示上次数据";
  } finally {
    if (request === requestSerial) loading.value = false;
  }
}

function formatFetchedAt(value: number): string {
  return new Date(value * 1000).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
</script>

<template>
  <section class="map-events-page" aria-label="剑网3地图事件">
    <header class="map-events-page__header">
      <button type="button" aria-label="返回快捷面板" @click="$emit('back')">←</button>
      <strong>地图事件</strong>
      <button type="button" aria-label="关闭快捷面板" @click="$emit('close')">×</button>
    </header>

    <div class="map-events-page__toolbar">
      <span>{{ view?.source ?? "JX3API（第三方）" }}</span>
      <button type="button" :disabled="loading" @click="loadCategory(true)">
        {{ loading ? "加载中" : "刷新" }}
      </button>
    </div>

    <div class="category-tabs" aria-label="地图事件分类">
      <button
        v-for="category in CATEGORIES"
        :key="category.name"
        type="button"
        :class="{ 'category-tabs__button--active': selectedCategory === category.name }"
        @click="selectCategory(category.name)"
      >
        <span>{{ category.marker }}</span>
        <strong>{{ category.name }}</strong>
      </button>
    </div>

    <p v-if="staleMessage" class="map-events-page__notice">{{ staleMessage }}</p>
    <p
      v-if="errorMessage && view"
      class="map-events-page__notice map-events-page__notice--error"
    >
      {{ errorMessage }}
    </p>

    <div v-if="!view && loading" class="map-events-page__state">
      正在读取{{ selectedCategory }}排期…
    </div>
    <div v-else-if="!view" class="map-events-page__state">
      <span>{{ errorMessage || "暂时无法读取地图事件" }}</span>
      <button type="button" @click="loadCategory(true)">重试</button>
    </div>
    <div v-else-if="view.items.length === 0" class="map-events-page__state">
      暂无{{ selectedCategory }}排期
    </div>
    <ol v-else class="event-list">
      <li v-for="item in view.items" :key="item.id">
        <time>{{ item.time }}</time>
        <article>
          <strong>{{ item.stage }}</strong>
          <small>{{ item.map }} · {{ item.site }}</small>
          <p>{{ item.description }}</p>
        </article>
      </li>
    </ol>

    <footer v-if="view" class="map-events-page__footer">
      数据更新于 {{ formatFetchedAt(view.fetchedAt) }}
    </footer>
  </section>
</template>

<style scoped>
.map-events-page {
  display: grid;
  grid-template-rows: auto auto auto auto minmax(0, 1fr) auto;
  gap: 8px;
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

.map-events-page__header,
.map-events-page__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.map-events-page__header strong {
  font-size: 16px;
}

.map-events-page button {
  border: 1px solid rgb(217 119 6 / 20%);
  border-radius: 10px;
  color: #6b4423;
  background: rgb(255 255 255 / 78%);
  font: inherit;
  cursor: pointer;
}

.map-events-page__header button {
  width: 30px;
  height: 30px;
  padding: 0;
  font-size: 19px;
}

.map-events-page__toolbar {
  color: #8a6848;
  font-size: 10px;
}

.map-events-page__toolbar button {
  padding: 6px 10px;
}

.category-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 5px;
}

.category-tabs button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  min-width: 0;
  padding: 6px;
}

.category-tabs button span {
  display: grid;
  width: 20px;
  height: 20px;
  border-radius: 7px;
  place-items: center;
  color: #fff;
  background: #a07145;
  font-size: 9px;
  font-weight: 700;
}

.category-tabs button strong {
  font-size: 10px;
}

.category-tabs .category-tabs__button--active {
  border-color: rgb(217 119 6 / 50%);
  color: #92400e;
  background: #fde68a;
}

.category-tabs .category-tabs__button--active span {
  background: #d97706;
}

.map-events-page__notice {
  min-height: 14px;
  margin: 0;
  color: #8a5a24;
  font-size: 9px;
  text-align: center;
}

.map-events-page__notice--error {
  color: #b91c1c;
}

.map-events-page__state {
  display: grid;
  min-height: 0;
  place-content: center;
  gap: 10px;
  color: #8a6848;
  font-size: 12px;
  text-align: center;
}

.map-events-page__state button {
  padding: 7px 12px;
}

.event-list {
  min-height: 0;
  margin: 0;
  padding: 0 2px 0 0;
  overflow-y: auto;
  list-style: none;
}

.event-list li {
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr);
  align-items: start;
  gap: 7px;
}

.event-list li + li {
  margin-top: 6px;
}

.event-list time {
  border-radius: 9px;
  padding: 7px 3px;
  color: #fff;
  background: #d97706;
  font-size: 10px;
  font-weight: 700;
  text-align: center;
}

.event-list article {
  min-width: 0;
  border: 1px solid rgb(217 119 6 / 12%);
  border-radius: 10px;
  padding: 7px 9px;
  background: rgb(255 255 255 / 68%);
}

.event-list article strong,
.event-list article small {
  display: block;
}

.event-list article strong {
  font-size: 11px;
  line-height: 1.4;
}

.event-list article small {
  margin-top: 2px;
  color: #9a5b18;
  font-size: 9px;
}

.event-list article p {
  margin: 5px 0 0;
  color: #705138;
  font-size: 9px;
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.map-events-page__footer {
  color: #9a7653;
  font-size: 9px;
  text-align: center;
}

button:disabled {
  cursor: default;
  opacity: 0.45;
}

button:focus-visible {
  outline: 3px solid rgb(245 158 11 / 45%);
  outline-offset: 2px;
}
</style>
