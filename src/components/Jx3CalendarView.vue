<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import {
  fetchCalendar,
  getCachedCalendar,
  type CalendarDay,
  type CalendarView,
} from "@/platform/jx3";

defineEmits<{
  back: [];
  close: [];
}>();

const calendar = ref<CalendarView>();
const selectedDate = ref("");
const loading = ref(false);
const errorMessage = ref("");
const staleMessage = ref("");

const DAILY_CATEGORIES = new Set(["大战", "战场", "阵营矿车", "门派事件", "驰援"]);
const CATEGORY_META: Record<string, { marker: string; tone: string }> = {
  大战: { marker: "战", tone: "red" },
  战场: { marker: "场", tone: "orange" },
  阵营矿车: { marker: "矿", tone: "amber" },
  门派事件: { marker: "派", tone: "violet" },
  驰援: { marker: "援", tone: "teal" },
  美人图: { marker: "美", tone: "rose" },
  宠物福缘: { marker: "宠", tone: "green" },
  "家园声望·加倍道具": { marker: "园", tone: "sky" },
  世界boss: { marker: "世", tone: "red" },
  "武林通鉴·公共任务": { marker: "公", tone: "blue" },
  "武林通鉴·团队秘境": { marker: "团", tone: "indigo" },
};

const selectedDay = computed<CalendarDay | undefined>(() =>
  calendar.value?.days.find((day) => day.date === selectedDate.value),
);
const selectedGroups = computed(() => {
  const groups = new Map<string, CalendarDay["items"]>();
  for (const item of selectedDay.value?.items ?? []) {
    const group = groups.get(item.category) ?? [];
    group.push(item);
    groups.set(item.category, group);
  }
  return Array.from(groups, ([category, items]) => ({
    category,
    items,
    marker: CATEGORY_META[category]?.marker ?? category.slice(0, 1),
    tone: CATEGORY_META[category]?.tone ?? "neutral",
  }));
});
const dailyGroups = computed(() =>
  selectedGroups.value.filter((group) => DAILY_CATEGORIES.has(group.category)),
);
const extraGroups = computed(() =>
  selectedGroups.value.filter((group) => !DAILY_CATEGORIES.has(group.category)),
);

onMounted(() => {
  void loadCalendar();
});

async function loadCalendar(force = false): Promise<void> {
  if (loading.value) return;
  loading.value = true;
  errorMessage.value = "";
  staleMessage.value = "";
  let cached: CalendarView | null = null;
  try {
    if (!force) {
      cached = await getCachedCalendar();
      if (cached) {
        applyCalendar(cached);
        if (!cached.stale) return;
        staleMessage.value = "正在更新上次数据…";
      }
    }
    const refreshed = await fetchCalendar(force);
    applyCalendar(refreshed);
    staleMessage.value = refreshed.stale ? "当前显示上次数据" : "";
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
    if (cached || calendar.value) staleMessage.value = "刷新失败，当前显示上次数据";
  } finally {
    loading.value = false;
  }
}

function applyCalendar(value: CalendarView): void {
  calendar.value = value;
  if (!value.days.some((day) => day.date === selectedDate.value)) {
    selectedDate.value = value.days[0]?.date ?? "";
  }
}

function dateLabel(day: CalendarDay): string {
  if (!day.predicted) return "今天";
  return `周${day.weekday}`;
}

function dayNumber(date: string): string {
  const [, month = "", day = ""] = date.split("-");
  return `${Number(month)}/${Number(day)}`;
}

function fullDate(day: CalendarDay): string {
  const [year = "", month = "", date = ""] = day.date.split("-");
  return `${year}年${Number(month)}月${Number(date)}日 · 星期${day.weekday}`;
}

function formatFetchedAt(value: number): string {
  return new Date(value * 1000).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function groupItemCount(groups: typeof selectedGroups.value): number {
  return groups.reduce((total, group) => total + group.items.length, 0);
}
</script>

<template>
  <section class="calendar-page" aria-label="剑网3日历">
    <header class="calendar-page__header">
      <button type="button" aria-label="返回快捷面板" @click="$emit('back')">←</button>
      <strong>日历与今日活动</strong>
      <button type="button" aria-label="关闭快捷面板" @click="$emit('close')">×</button>
    </header>

    <div class="calendar-page__toolbar">
      <span>{{ calendar?.source ?? "JX3API（第三方）" }}</span>
      <button type="button" :disabled="loading" @click="loadCalendar(true)">
        {{ loading ? "加载中" : "刷新" }}
      </button>
    </div>

    <div v-if="calendar" class="date-strip" aria-label="选择日期">
      <button
        v-for="day in calendar.days"
        :key="day.date"
        type="button"
        :class="{ 'date-strip__button--active': day.date === selectedDate }"
        @click="selectedDate = day.date"
      >
        <strong>{{ dateLabel(day) }}</strong>
        <small>{{ dayNumber(day.date) }}</small>
      </button>
    </div>

    <div v-if="!calendar && loading" class="calendar-page__state">
      正在连接 JX3API…
    </div>
    <div v-else-if="!calendar" class="calendar-page__state">
      <span>{{ errorMessage || "暂时无法读取日历" }}</span>
      <button type="button" @click="loadCalendar(true)">重试</button>
    </div>
    <article v-else-if="selectedDay" class="calendar-day">
      <header>
        <span>
          <strong>{{ fullDate(selectedDay) }}</strong>
          <small v-if="selectedDay.predicted">未来内容 · 预测</small>
          <small v-else>今日数据 · 第三方</small>
        </span>
      </header>

      <p v-if="staleMessage" class="calendar-page__notice">{{ staleMessage }}</p>
      <p v-if="calendar.incomplete" class="calendar-page__notice">
        数据可能不完整
      </p>
      <p v-if="errorMessage" class="calendar-page__notice calendar-page__notice--error">
        {{ errorMessage }}
      </p>

      <div v-if="selectedDay.items.length === 0" class="calendar-page__empty">
        这一天暂无可用数据
      </div>
      <div v-else class="calendar-content">
        <section v-if="dailyGroups.length" class="activity-section">
          <header class="activity-section__header">
            <span><b>日</b><strong>日常活动</strong></span>
            <small>{{ groupItemCount(dailyGroups) }} 项</small>
          </header>
          <div class="activity-grid">
            <article
              v-for="group in dailyGroups"
              :key="group.category"
              class="activity-card"
              :class="[
                `activity-card--${group.tone}`,
                { 'activity-card--wide': group.category === '大战' },
              ]"
            >
              <header>
                <span>{{ group.marker }}</span>
                <strong>{{ group.category }}</strong>
              </header>
              <div class="activity-tags">
                <span v-for="item in group.items" :key="item.id">{{ item.name }}</span>
              </div>
            </article>
          </div>
        </section>

        <section v-if="extraGroups.length || selectedDay.weeklyPending" class="activity-section">
          <header class="activity-section__header">
            <span><b>轮</b><strong>轮换与周常</strong></span>
            <small>
              {{ groupItemCount(extraGroups) }} 项
              <template v-if="selectedDay.weeklyPending">· 周常待公布</template>
            </small>
          </header>
          <div class="activity-grid">
            <article
              v-for="group in extraGroups"
              :key="group.category"
              class="activity-card"
              :class="[
                `activity-card--${group.tone}`,
                { 'activity-card--wide': group.items.length >= 3 },
              ]"
            >
              <header>
                <span>{{ group.marker }}</span>
                <strong>{{ group.category }}</strong>
              </header>
              <div class="activity-tags">
                <span v-for="item in group.items" :key="item.id">{{ item.name }}</span>
              </div>
            </article>
            <article
              v-if="selectedDay.weeklyPending"
              class="activity-card activity-card--pending activity-card--wide"
            >
              <header>
                <span>周</span>
                <strong>下周武林通鉴</strong>
              </header>
              <p>暂未公布</p>
            </article>
          </div>
        </section>
      </div>
    </article>

    <footer v-if="calendar" class="calendar-page__footer">
      数据更新于 {{ formatFetchedAt(calendar.fetchedAt) }}
    </footer>
  </section>
</template>

<style scoped>
.calendar-page {
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

.calendar-page__header,
.calendar-page__toolbar,
.calendar-day > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.calendar-page__header strong {
  font-size: 16px;
}

.calendar-page button {
  border: 1px solid rgb(217 119 6 / 20%);
  border-radius: 10px;
  color: #6b4423;
  background: rgb(255 255 255 / 78%);
  font: inherit;
  cursor: pointer;
}

.calendar-page__header button {
  width: 30px;
  height: 30px;
  padding: 0;
  font-size: 19px;
}

.calendar-page__toolbar {
  color: #8a6848;
  font-size: 10px;
}

.calendar-page__toolbar button {
  padding: 6px 10px;
}

.date-strip {
  display: grid;
  grid-template-columns: repeat(8, minmax(0, 1fr));
  gap: 3px;
}

.date-strip button {
  display: grid;
  min-width: 0;
  gap: 1px;
  padding: 5px 2px;
  text-align: center;
}

.date-strip button strong {
  font-size: 10px;
}

.date-strip button small {
  color: #9a7653;
  font-size: 9px;
}

.date-strip .date-strip__button--active {
  border-color: rgb(217 119 6 / 50%);
  color: #92400e;
  background: #fde68a;
}

.calendar-day {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.calendar-day > header {
  margin-bottom: 6px;
}

.calendar-day > header span,
.calendar-day > header strong,
.calendar-day > header small {
  display: block;
}

.calendar-day > header strong {
  font-size: 12px;
}

.calendar-day > header small {
  margin-top: 2px;
  color: #9a5b18;
  font-size: 9px;
}

.calendar-content {
  min-height: 0;
  flex: 1;
  padding-right: 2px;
  overflow-y: auto;
}

.activity-section + .activity-section {
  margin-top: 10px;
}

.activity-section__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 5px;
}

.activity-section__header > span {
  display: flex;
  align-items: center;
  gap: 5px;
}

.activity-section__header b {
  display: grid;
  width: 18px;
  height: 18px;
  border-radius: 6px;
  place-items: center;
  color: #fff;
  background: #d97706;
  font-size: 9px;
}

.activity-section__header strong {
  font-size: 11px;
}

.activity-section__header small {
  color: #a07145;
  font-size: 9px;
}

.activity-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 5px;
}

.activity-card {
  --card-accent: #a07145;
  --card-tint: rgb(255 255 255 / 70%);

  min-width: 0;
  border: 1px solid color-mix(in srgb, var(--card-accent) 16%, transparent);
  border-radius: 10px;
  padding: 7px;
  background: var(--card-tint);
}

.activity-card--wide {
  grid-column: 1 / -1;
}

.activity-card > header {
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
  margin-bottom: 5px;
}

.activity-card > header span {
  display: grid;
  flex: 0 0 20px;
  width: 20px;
  height: 20px;
  border-radius: 7px;
  place-items: center;
  color: #fff;
  background: var(--card-accent);
  font-size: 9px;
  font-weight: 700;
}

.activity-card > header strong {
  min-width: 0;
  font-size: 10px;
  line-height: 1.3;
}

.activity-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.activity-tags span {
  max-width: 100%;
  border-radius: 6px;
  padding: 3px 5px;
  color: #4b3421;
  background: rgb(255 255 255 / 75%);
  font-size: 9px;
  line-height: 1.35;
  overflow-wrap: anywhere;
}

.activity-card--red {
  --card-accent: #c2410c;
  --card-tint: rgb(255 237 213 / 72%);
}

.activity-card--orange {
  --card-accent: #ea580c;
  --card-tint: rgb(255 247 237 / 75%);
}

.activity-card--amber {
  --card-accent: #ca8a04;
  --card-tint: rgb(254 249 195 / 68%);
}

.activity-card--violet {
  --card-accent: #7c3aed;
  --card-tint: rgb(245 243 255 / 74%);
}

.activity-card--teal {
  --card-accent: #0f766e;
  --card-tint: rgb(240 253 250 / 72%);
}

.activity-card--rose {
  --card-accent: #e11d48;
  --card-tint: rgb(255 241 242 / 74%);
}

.activity-card--green {
  --card-accent: #15803d;
  --card-tint: rgb(240 253 244 / 74%);
}

.activity-card--sky {
  --card-accent: #0369a1;
  --card-tint: rgb(240 249 255 / 74%);
}

.activity-card--blue {
  --card-accent: #1d4ed8;
  --card-tint: rgb(239 246 255 / 74%);
}

.activity-card--indigo {
  --card-accent: #4338ca;
  --card-tint: rgb(238 242 255 / 74%);
}

.activity-card--pending {
  --card-accent: #78716c;
  --card-tint: rgb(250 250 249 / 62%);

  border-style: dashed;
}

.activity-card--pending p {
  margin: 0;
  color: #8a6848;
  font-size: 9px;
}

.calendar-page__notice {
  margin: 4px 0;
  color: #8a5a24;
  font-size: 10px;
  text-align: center;
}

.calendar-page__notice--error {
  color: #b91c1c;
}

.calendar-page__state,
.calendar-page__empty {
  display: grid;
  min-height: 0;
  place-content: center;
  gap: 10px;
  color: #8a6848;
  font-size: 12px;
  text-align: center;
}

.calendar-page__state button {
  padding: 7px 12px;
}

.calendar-page__footer {
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
