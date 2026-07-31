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

const selectedDay = computed<CalendarDay | undefined>(() =>
  calendar.value?.days.find((day) => day.date === selectedDate.value),
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
      <ul v-else>
        <li v-for="item in selectedDay.items" :key="item.id">
          <span>{{ item.category }}</span>
          <strong>{{ item.name }}</strong>
        </li>
      </ul>
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

.calendar-day ul {
  min-height: 0;
  flex: 1;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.calendar-day li {
  display: grid;
  grid-template-columns: 58px minmax(0, 1fr);
  align-items: center;
  gap: 8px;
  border-radius: 10px;
  padding: 8px 9px;
  background: rgb(255 255 255 / 65%);
}

.calendar-day li + li {
  margin-top: 5px;
}

.calendar-day li span {
  color: #9a5b18;
  font-size: 10px;
}

.calendar-day li strong {
  overflow: hidden;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
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
