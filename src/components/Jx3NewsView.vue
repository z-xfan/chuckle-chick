<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import {
  fetchNewsPage,
  getCachedNewsPage,
  openOfficialNews,
  type NewsKind,
  type NewsPage,
} from "@/platform/jx3";

const props = defineProps<{
  kind: NewsKind;
}>();

defineEmits<{
  back: [];
  close: [];
}>();

const currentPage = ref(1);
const pageData = ref<NewsPage>();
const loading = ref(false);
const errorMessage = ref("");
const staleMessage = ref("");

const title = computed(() => (props.kind === "announcement" ? "官方公告" : "武学调整"));

onMounted(() => {
  void loadPage(1);
});

async function loadPage(page: number, force = false): Promise<void> {
  if (loading.value || page < 1) return;
  loading.value = true;
  errorMessage.value = "";
  staleMessage.value = "";
  let cached: NewsPage | null = null;
  try {
    if (!force) {
      cached = await getCachedNewsPage(props.kind, page);
      if (cached) {
        pageData.value = cached;
        currentPage.value = page;
        if (!cached.stale) return;
        staleMessage.value = "正在更新上次数据…";
      }
    }
    pageData.value = await fetchNewsPage(props.kind, page, force);
    currentPage.value = page;
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
    if (cached) staleMessage.value = "刷新失败，当前显示上次数据";
  } finally {
    loading.value = false;
  }
}

async function openItem(url: string): Promise<void> {
  errorMessage.value = "";
  try {
    await openOfficialNews(url);
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

function formatTime(value: string): string {
  return value.replace("T", " ").slice(0, 16);
}
</script>

<template>
  <section class="jx3-page" :aria-label="title">
    <header class="jx3-page__header">
      <button type="button" aria-label="返回快捷面板" @click="$emit('back')">←</button>
      <strong>{{ title }}</strong>
      <button type="button" aria-label="关闭快捷面板" @click="$emit('close')">×</button>
    </header>

    <div class="jx3-page__toolbar">
      <span>第 {{ currentPage }} 页</span>
      <button type="button" :disabled="loading" @click="loadPage(currentPage, true)">
        {{ loading ? "加载中" : "刷新" }}
      </button>
    </div>

    <p v-if="staleMessage" class="jx3-page__notice">{{ staleMessage }}</p>
    <p v-if="errorMessage" class="jx3-page__notice jx3-page__notice--error">
      {{ errorMessage }}
    </p>

    <div v-if="!pageData && loading" class="jx3-page__state">正在连接西山居官网…</div>
    <div v-else-if="!pageData" class="jx3-page__state">
      <span>暂时无法读取{{ title }}</span>
      <button type="button" @click="loadPage(currentPage, true)">重试</button>
    </div>
    <div v-else-if="pageData.items.length === 0" class="jx3-page__state">
      当前页面没有内容
    </div>
    <ol v-else class="news-list">
      <li v-for="item in pageData.items" :key="item.id">
        <button type="button" @click="openItem(item.sourceUrl)">
          <span class="news-list__title">
            <i v-if="item.isSkillChange">技改</i>
            {{ item.title }}
          </span>
          <time>{{ formatTime(item.publishedAt) }}</time>
        </button>
      </li>
    </ol>

    <footer class="jx3-page__pagination">
      <button
        type="button"
        :disabled="loading || currentPage <= 1"
        @click="loadPage(currentPage - 1)"
      >
        上一页
      </button>
      <span>
        {{ currentPage
        }}<template v-if="pageData?.totalPages"> / {{ pageData.totalPages }}</template>
      </span>
      <button
        type="button"
        :disabled="loading || !pageData?.hasMore"
        @click="loadPage(currentPage + 1)"
      >
        下一页
      </button>
    </footer>
  </section>
</template>

<style scoped>
.jx3-page {
  display: grid;
  grid-template-rows: auto auto auto minmax(0, 1fr) auto;
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

.jx3-page__header,
.jx3-page__toolbar,
.jx3-page__pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.jx3-page__header strong {
  font-size: 16px;
}

.jx3-page button {
  border: 1px solid rgb(217 119 6 / 20%);
  border-radius: 10px;
  color: #6b4423;
  background: rgb(255 255 255 / 78%);
  font: inherit;
  cursor: pointer;
}

.jx3-page__header button {
  width: 30px;
  height: 30px;
  padding: 0;
  font-size: 19px;
}

.jx3-page__toolbar {
  color: #8a6848;
  font-size: 11px;
}

.jx3-page__toolbar button,
.jx3-page__pagination button {
  padding: 6px 10px;
}

.jx3-page__notice {
  min-height: 16px;
  margin: 0;
  color: #8a5a24;
  font-size: 11px;
  text-align: center;
}

.jx3-page__notice--error {
  color: #b91c1c;
}

.jx3-page__state {
  display: grid;
  min-height: 0;
  place-content: center;
  gap: 10px;
  color: #8a6848;
  font-size: 12px;
  text-align: center;
}

.jx3-page__state button {
  padding: 7px 12px;
}

.news-list {
  min-height: 0;
  margin: 0;
  padding: 0;
  overflow: auto;
  list-style: none;
}

.news-list li + li {
  margin-top: 6px;
}

.news-list button {
  display: grid;
  width: 100%;
  gap: 4px;
  padding: 8px 10px;
  text-align: left;
}

.news-list__title {
  display: -webkit-box;
  overflow: hidden;
  font-size: 12px;
  font-weight: 700;
  line-height: 1.4;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.news-list__title i {
  display: inline-block;
  margin-right: 4px;
  border-radius: 5px;
  padding: 1px 4px;
  color: #92400e;
  background: #fde68a;
  font-size: 9px;
  font-style: normal;
}

.news-list time {
  color: #9a7653;
  font-size: 10px;
}

.jx3-page__pagination {
  font-size: 11px;
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
