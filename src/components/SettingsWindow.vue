<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import Icon from "./AppIcon.vue";
import HolidayPanel from "./HolidayPanel.vue";
import { getWorkCountdown } from "../countdown";
import { defaultSettings } from "../defaults";
import type { Snapshot } from "../types";

const connected = isTauri();
const settings = ref(structuredClone(defaultSettings));
const snapshot = ref<Snapshot | null>(null);
const busy = ref(false);
const error = ref("");
const toast = ref("");
const now = ref(Date.now());
const workClock = computed(() =>
  getWorkCountdown(snapshot.value?.settings ?? defaultSettings, now.value),
);
const workClockLabel = computed(() =>
  workClock.value.state === "overtime" ? "已加班" :
    workClock.value.state === "dayOff" ? "休息日" : "距离下班",
);
const dirty = computed(
  () =>
    snapshot.value &&
    JSON.stringify(settings.value) !== JSON.stringify(snapshot.value.settings),
);
const weekdays = ["一", "二", "三", "四", "五", "六", "日"];
const phaseLabel = computed(
  () =>
    ({
      scheduled: "提醒已安排",
      due: "已到提醒时间",
      paused: "今日已暂停",
      dayOff: "今天休息",
      finished: "今日提醒已结束",
    })[snapshot.value?.phase || "scheduled"],
);
const nextLabel = computed(() =>
  snapshot.value?.nextAt
    ? new Date(snapshot.value.nextAt * 1000).toLocaleTimeString("zh-CN", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      })
    : "— — : — —",
);
const countdown = computed(() => {
  const next = snapshot.value?.nextAt;
  if (!next) return snapshot.value?.phase === "finished" ? "今天没有后续提醒" : "安心享受自己的时间";
  const minutes = Math.ceil((next * 1000 - now.value) / 60000);
  if (minutes <= 0) return "到点了，准备告别今天的工作";
  return minutes >= 60
    ? `还有 ${Math.floor(minutes / 60)} 小时 ${minutes % 60} 分钟`
    : `还有 ${minutes} 分钟`;
});
const today = computed(() =>
  new Date(now.value).toLocaleDateString("zh-CN", {
    month: "long",
    day: "numeric",
    weekday: "long",
  }),
);
let interval: ReturnType<typeof setInterval>;
let toastTimer: ReturnType<typeof setTimeout>;
const unlisteners: UnlistenFn[] = [];

async function refresh(initial = false) {
  if (!connected) return;
  try {
    const data = await invoke<Snapshot>("get_snapshot");
    snapshot.value = data;
    if (initial) settings.value = structuredClone(data.settings);
  } catch (e) {
    error.value = String(e);
  }
}
function notify(message: string) {
  toast.value = message;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toast.value = ""), 3500);
}
async function run(operation: () => Promise<void>) {
  if (busy.value || !connected) return;
  busy.value = true;
  error.value = "";
  try {
    await operation();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
async function save() {
  await run(async () => {
    const data = await invoke<Snapshot>("save_settings", {
      settings: JSON.parse(JSON.stringify(settings.value)),
    });
    snapshot.value = data;
    settings.value = structuredClone(data.settings);
    notify("设置已保存，新的提醒安排已生效");
  });
}
async function action(value: "pause" | "resume") {
  await run(async () => {
    snapshot.value = await invoke<Snapshot>("reminder_action", {
      action: value,
    });
    notify(
      value === "pause" ? "今天不再提醒，明天自动恢复" : "今日提醒已恢复",
    );
  });
}
async function test() {
  await run(async () => {
    await invoke("test_reminder");
    notify("已发送测试提醒，使用的是已保存的设置");
  });
}
async function toggleCountdown() {
  await run(async () => {
    const visible = !snapshot.value?.countdownVisible;
    await invoke("set_countdown_visibility", { visible });
    await refresh();
    notify(visible ? "下班倒计时已显示，可拖动移动" : "倒计时已隐藏，可从托盘重新打开");
  });
}

onMounted(async () => {
  clockInterval = setInterval(() => { now.value = Date.now(); }, 1000);
  if (connected) {
    await refresh(true);
    unlisteners.push(await listen("state-changed", () => refresh()));
    unlisteners.push(await listen("reminder-shown", () => refresh()));
    interval = setInterval(() => void refresh(), 5000);
  }
});
let clockInterval: ReturnType<typeof setInterval>;
onUnmounted(() => {
  clearInterval(interval);
  clearInterval(clockInterval);
  clearTimeout(toastTimer);
  unlisteners.forEach((fn) => fn());
});
</script>

<template>
  <main class="app-shell">
    <header class="page-header">
      <div class="brand">
        <div class="brand-symbol"><Icon name="sun" :size="28" /></div>
        <div>
          <h1>下班啦<span>OFFWORK</span></h1>
          <p>工作有时，生活也是。</p>
        </div>
      </div>
      <div class="header-date"><span class="live-dot" /> {{ today }}</div>
    </header>
    <div v-if="!connected" class="notice">
      当前为界面预览。请在桌面应用中设置和接收提醒。
    </div>
    <div v-if="error || snapshot?.warning" class="notice error" role="alert">
      {{ error || snapshot?.warning }}
    </div>

    <section class="hero" :data-phase="workClock.state" aria-label="今日计时">
      <div class="hero-copy">
        <span class="eyebrow"><Icon name="sun" :size="15" /> 给生活，留一点时间</span>
        <h2>今日计时</h2>
      </div>
      <div class="hero-art" aria-hidden="true">
      </div>
      <div class="hero-clock">
        <span class="hero-clock-label"><span class="tiny-dot" />{{ workClockLabel }}</span>
        <p v-if="workClock.state !== 'dayOff'" class="hero-clock-value">{{ workClock.value }}</p>
        <p v-else class="hero-clock-value day-off">今天休息</p>
      </div>
    </section>

    <HolidayPanel :now="now" />

    <div class="workspace-grid">
      <form class="settings-card" @submit.prevent="save">
        <div class="section-heading">
          <div>
            <h2><Icon name="clock" /> 提醒安排</h2>
          </div>
          <label class="sedentary-setting">久坐提醒
            <select v-model.number="settings.sedentaryMinutes" aria-label="久坐提醒间隔">
              <option :value="0">关闭</option>
              <option v-for="minutes in [30, 45, 60]" :key="minutes" :value="minutes">每 {{ minutes }} 分钟</option>
            </select>
          </label>
        </div>
        <div class="time-fields">
          <label
            >下班时间<input
              v-model="settings.startTime"
              type="time"
              required
              aria-label="下班时间" /></label
          ><span class="time-separator">至</span
          ><label
            >停止提醒时间<input
              v-model="settings.endTime"
              type="time"
              required
              aria-label="停止提醒时间"
          /></label>
        </div>
        <div class="interval-row">
          <div>
            <label for="interval">重复提醒间隔</label>
          </div>
          <div class="number-field">
            <input
              id="interval"
              v-model.number="settings.intervalMinutes"
              type="number"
              min="1"
              max="1440"
              step="1"
              required
            /><span>分钟</span>
          </div>
        </div>
        <fieldset class="days-field">
          <legend>哪些天提醒</legend>
          <div class="day-buttons">
            <button
              v-for="(day, index) in weekdays"
              :key="day"
              type="button"
              :class="{ selected: settings.days[index] }"
              :aria-pressed="settings.days[index]"
              :aria-label="`星期${day}`"
              @click="settings.days[index] = !settings.days[index]"
            >
              {{ day }}
            </button>
          </div>
        </fieldset>
        <div class="divider" />
        <label class="message-label" for="message"
          >提醒文案<span>{{ settings.message.length }} / 200</span></label
        ><textarea
          id="message"
          v-model="settings.message"
          maxlength="200"
          rows="2"
          required
          placeholder="写一句想在下班时对自己说的话…"
        />
        <div class="settings-toggles">
          <div class="toggle-row">
            <span class="option-icon"><Icon name="volume" /></span>
            <label for="sound">提示音</label>
            <input
              id="sound"
              v-model="settings.sound"
              type="checkbox"
              class="switch"
              role="switch"
            />
          </div>
          <div class="toggle-row">
            <span class="option-icon"><Icon name="power" /></span>
            <label for="autostart">登录后自动启动</label>
            <input
              id="autostart"
              v-model="settings.autostart"
              type="checkbox"
              class="switch"
              role="switch"
            />
          </div>
        </div>
        <div class="form-footer">
          <span :class="{ unsaved: dirty }"
            ><span class="tiny-dot" />{{
              dirty ? "有尚未保存的修改" : "设置仅保存在这台电脑"
            }}</span
          ><button class="primary" type="submit" :disabled="busy || !connected">
            <Icon name="check" :size="17" />{{ busy ? "处理中…" : "保存设置" }}
          </button>
        </div>
      </form>

      <aside class="sidebar">
        <section class="preview-card widget-card">
          <div class="section-heading">
            <h3>桌面悬浮窗</h3>
            <Icon name="clock" :size="19" />
          </div>
          <button
            class="secondary full-width"
            :disabled="busy || !snapshot"
            @click="toggleCountdown"
          >{{ snapshot?.countdownVisible ? '隐藏悬浮窗' : '显示悬浮窗' }}</button>
        </section>
        <section class="status-card">
          <div class="status-top">
            <span class="eyebrow">下一次提醒</span
            ><span class="status-pill" :class="{ muted: snapshot?.paused }"
              ><span class="tiny-dot" />{{ phaseLabel }}</span
            >
          </div>
          <div class="next-time">{{ nextLabel }}</div>
          <p class="countdown">{{ countdown }}</p>
          <div class="status-line" />
          <button
            class="pause-button"
            :disabled="busy || !connected"
            @click="action(snapshot?.paused ? 'resume' : 'pause')"
          >
            <Icon :name="snapshot?.paused ? 'play' : 'pause'" :size="16" />{{
              snapshot?.paused ? "恢复今日提醒" : "今天不再提醒"
            }}
          </button>
        </section>
        <section class="preview-card">
          <div class="section-heading">
            <h3>预览下班提醒</h3>
            <Icon name="bell" :size="19" />
          </div>
          <p>提醒显示 15 秒后自动收起。</p>
          <button
            class="secondary full-width"
            :disabled="busy || !connected"
            @click="test"
          >
            <Icon name="play" :size="16" /> 测试提醒</button
          ><span v-if="dirty" class="test-note">测试使用已保存的设置</span>
        </section>
      </aside>
    </div>
    <footer class="page-footer">
      <span
        ><Icon name="info" :size="14" />关闭窗口后，提醒仍在托盘中运行。</span
      ><span>下班啦 <span class="footer-dot">·</span> v0.5.1</span>
    </footer>
    <Transition name="toast"
      ><div v-if="toast" class="toast-message" role="status">
        <Icon name="check" :size="18" />{{ toast }}
      </div></Transition
    >
  </main>
</template>
