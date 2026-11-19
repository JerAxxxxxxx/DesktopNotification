<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getWorkCountdown } from "../countdown";
import type { Settings, Snapshot } from "../types";
import Icon from "./AppIcon.vue";

const now = ref(Date.now());
const settings = ref<Settings>();
const failed = ref(false);
const clock = computed(() => settings.value && getWorkCountdown(settings.value, now.value));
const caption = computed(() => failed.value ? "悬浮窗操作失败" :
  clock.value?.state === "overtime" ? "已加班" :
    clock.value?.state === "dayOff" ? "休息日" : "距离下班");
let ticker: ReturnType<typeof setInterval>;
let unlisten: UnlistenFn | undefined;
let dragFrame: number | undefined;
let grip: { id: number; x: number; y: number; screenX: number; screenY: number } | undefined;

async function refresh() {
  now.value = Date.now();
  try {
    settings.value = (await invoke<Snapshot>("get_snapshot")).settings;
    failed.value = false;
  } catch {
    failed.value = true;
  }
}

function beginDrag(event: PointerEvent) {
  if (!isTauri() || event.button !== 0 || grip) return;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  grip = {
    id: event.pointerId,
    x: event.clientX,
    y: event.clientY,
    screenX: event.screenX,
    screenY: event.screenY,
  };
}

function moveDrag(event: PointerEvent) {
  if (!grip || grip.id !== event.pointerId) return;
  if (event.screenX === grip.screenX && event.screenY === grip.screenY) return;
  const { x, y } = grip;
  grip.screenX = event.screenX;
  grip.screenY = event.screenY;
  if (dragFrame !== undefined) return;
  dragFrame = requestAnimationFrame(() => {
    dragFrame = undefined;
    void invoke("move_countdown", { x, y }).catch(() => { failed.value = true; });
  });
}

function finishDrag(event: PointerEvent) {
  if (grip?.id !== event.pointerId) return;
  if (event.type === "pointerup") moveDrag(event);
  grip = undefined;
}

async function hide() {
  try {
    await invoke("set_countdown_visibility", { visible: false });
  } catch {
    failed.value = true;
  }
}

onMounted(async () => {
  ticker = setInterval(() => { now.value = Date.now(); }, 1000);
  if (!isTauri()) return;
  try {
    unlisten = await listen("state-changed", refresh);
  } catch {
    failed.value = true;
  }
  await refresh();
});

onUnmounted(() => {
  clearInterval(ticker);
  if (dragFrame !== undefined) cancelAnimationFrame(dragFrame);
  unlisten?.();
});
</script>

<template>
  <main
    class="float-clock"
    :data-phase="clock?.state"
    aria-label="下班和加班计时"
    @contextmenu.prevent
  >
    <div
      class="float-grip"
      @pointerdown="beginDrag"
      @pointermove="moveDrag"
      @pointerup="finishDrag"
      @pointercancel="finishDrag"
      @lostpointercapture="finishDrag"
    >
      <span class="float-symbol"><Icon :name="clock?.state === 'dayOff' ? 'leaf' : clock?.state === 'overtime' ? 'moon' : 'sun'" :size="16" /></span>
      <div class="float-copy">
        <p class="float-caption">{{ caption }}</p>
        <p v-if="clock && clock.state !== 'dayOff'" class="float-time">{{ clock.value }}</p>
        <p v-else class="float-status">{{ clock?.state === 'dayOff' ? '今天休息' : failed ? '无法读取' : '正在读取…' }}</p>
      </div>
    </div>
    <button class="float-close" title="隐藏悬浮窗" aria-label="隐藏悬浮窗" @click="hide">
      <Icon name="close" :size="12" />
    </button>
  </main>
</template>

<style>
html.countdown-mode {
  background: #201d35;
  color: #fcf8ff;
  overflow: hidden;
}
.float-clock {
  --float-accent: #d0c1ff;
  --float-glow: #453967;
  position: relative;
  height: 100vh;
  display: flex;
  border: 1px solid #ffffff28;
  border-radius: 8px;
  background: radial-gradient(ellipse at 5% -80%, var(--float-glow) 25%, #201d35 75%);
  user-select: none;
}
.float-clock[data-phase="overtime"] {
  --float-accent: #ffd0a6;
  --float-glow: #694340;
}
.float-clock[data-phase="dayOff"] {
  --float-accent: #b5e8ce;
  --float-glow: #37585c;
}
.float-grip {
  cursor: grab;
  touch-action: none;
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  padding: 0 24px 0 8px;
}
.float-grip:active { cursor: grabbing; }
.float-symbol {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  flex: 0 0 auto;
  border: 1px solid #ffffff25;
  border-radius: 8px;
  color: var(--float-accent);
  background: #ffffff12;
}
.float-caption {
  color: var(--float-accent);
  font-size: 10px;
  font-weight: 650;
  letter-spacing: 0.12em;
}
.float-time {
  font-size: 22px;
  font-weight: 650;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.2px;
  line-height: 1.16;
}
.float-status { font-size: 16px; font-weight: 650; line-height: 1.4; }
.float-close {
  position: absolute;
  right: 3px;
  top: 3px;
  width: 20px;
  height: 20px;
  padding: 0;
  color: #e0d9ef;
  background: #ffffff0d;
  border-radius: 7px;
}
.float-close:hover { background: #ffffff24; }
</style>
