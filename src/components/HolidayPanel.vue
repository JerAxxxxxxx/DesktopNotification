<script setup lang="ts">
import { computed } from "vue";
import { getHolidayCountdowns, holidayYear } from "../holidays";
import Icon from "./AppIcon.vue";

const props = defineProps<{ now: number }>();
const holidays = computed(() => getHolidayCountdowns(props.now));
const featured = computed(() => holidays.value.find((holiday) => holiday.status !== "past"));

function shortDate(date: string) {
  return `${Number(date.slice(5, 7))} 月 ${Number(date.slice(8))} 日`;
}

function dateRange(start: string, end: string) {
  return start === end ? shortDate(start) : `${shortDate(start)}–${shortDate(end)}`;
}
</script>

<template>
  <details class="holiday-panel">
    <summary class="holiday-summary">
      <span class="holiday-icon"><Icon name="calendar" :size="20" /></span>
      <span class="holiday-intro">
        <strong>节假日倒计时</strong>
        <span>{{ holidayYear }} 年放假安排</span>
      </span>
      <span v-if="featured" class="holiday-featured">
        <span>{{ featured.name }}</span>
        <strong v-if="featured.status === 'upcoming'">还有 {{ featured.daysUntil }} 天</strong>
        <strong v-else>放假中</strong>
      </span>
      <span v-else class="holiday-featured">本年假期已结束</span>
      <Icon class="holiday-chevron" name="chevron" :size="16" />
    </summary>
    <ul class="holiday-list">
      <li v-for="holiday in holidays" :key="holiday.start" :data-status="holiday.status">
        <span class="holiday-name">{{ holiday.name }}</span>
        <span class="holiday-dates">{{ dateRange(holiday.start, holiday.end) }} · {{ holiday.duration }} 天</span>
        <span class="holiday-status">{{
          holiday.status === "upcoming" ? `还有 ${holiday.daysUntil} 天` :
            holiday.status === "ongoing" ? "放假中" : "已结束"
        }}</span>
      </li>
    </ul>
    <p class="holiday-note">
      按北京时间倒数到放假首日。来源：国办发明电〔2026〕10号。
    </p>
  </details>
</template>

<style scoped>
.holiday-panel {
  margin-bottom: 18px;
  border: 1px solid #e7dfee;
  border-radius: 15px;
  background: #fff;
  box-shadow: 0 4px 16px #3e235708;
}
.holiday-summary {
  display: flex;
  align-items: center;
  gap: 11px;
  min-height: 62px;
  padding: 9px 15px;
  cursor: pointer;
  list-style: none;
  border-radius: 14px;
}
.holiday-summary::-webkit-details-marker { display: none; }
.holiday-summary:hover { background: #fbf8fd; }
.holiday-summary:focus-visible { outline: 3px solid #a888de; outline-offset: 2px; }
.holiday-icon {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  flex: 0 0 auto;
  color: #65479c;
  background: #f1e9f9;
  border-radius: 10px;
}
.holiday-intro { display: grid; }
.holiday-intro > span,
.holiday-dates,
.holiday-note { font-size: 12px; color: #6e637d; }
.holiday-featured {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 10px;
  white-space: nowrap;
  color: #503b73;
}
.holiday-featured strong {
  padding: 3px 9px;
  border-radius: 8px;
  background: #f1e9f9;
  font-variant-numeric: tabular-nums;
}
.holiday-chevron { flex: 0 0 auto; color: #765aa8; }
.holiday-panel[open] .holiday-chevron { transform: rotate(180deg); }
.holiday-list { margin: 0 15px; padding: 0; list-style: none; }
.holiday-list li {
  display: grid;
  grid-template-columns: 4.5em minmax(0, 1fr) auto;
  align-items: center;
  gap: 8px;
  border-top: 1px solid #eee7f4;
  min-height: 37px;
}
.holiday-name { font-size: 13px; }
.holiday-status { font-size: 12px; color: #65479c; font-variant-numeric: tabular-nums; }
[data-status="past"] .holiday-status { color: #756b80; }
.holiday-note { margin: 8px 15px 12px; }
</style>
