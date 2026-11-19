<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { defaultSettings } from "../defaults";
import type { Snapshot } from "../types";
import Icon from "./AppIcon.vue";

const settings = ref(defaultSettings);
const busy = ref(false);
const error = ref("");
const connected = isTauri();
let refreshTimer: ReturnType<typeof setInterval>;
const unlisteners: UnlistenFn[] = [];

async function refresh() {
  try {
    settings.value = (await invoke<Snapshot>("get_snapshot")).settings;
  } catch (cause) {
    error.value = String(cause);
  }
}

async function run(command: string, args?: { action: "pause" | "snooze" }) {
  if (busy.value || !connected) return;
  busy.value = true;
  error.value = "";
  try {
    await invoke(command, args);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

onMounted(async () => {
  if (!connected) return;
  await refresh();
  unlisteners.push(await listen("state-changed", refresh));
  unlisteners.push(await listen("reminder-shown", refresh));
  refreshTimer = setInterval(() => void refresh(), 5000);
});

onUnmounted(() => {
  clearInterval(refreshTimer);
  unlisteners.forEach((unlisten) => unlisten());
});
</script>

<template>
  <main class="reminder-shell">
    <div class="reminder-top">
      <span class="brand-mini"><Icon name="sun" :size="18" /> 下班啦</span>
      <button class="icon-button" aria-label="关闭本次提醒" @click="run('dismiss_reminder')">
        <Icon name="close" :size="18" />
      </button>
    </div>
    <div class="reminder-symbol"><Icon name="sun" :size="30" /></div>
    <h1 class="reminder-title">今天的努力，到这里就好。</h1>
    <p class="reminder-message">{{ settings.message }}</p>
    <p v-if="error" class="reminder-error" role="alert">{{ error }}</p>
    <div class="reminder-buttons">
      <button class="primary" :disabled="busy" @click="run('reminder_action', { action: 'pause' })">
        <Icon name="check" :size="17" /> 今天不再提醒
      </button>
      <button class="secondary" :disabled="busy" @click="run('reminder_action', { action: 'snooze' })">
        {{ settings.intervalMinutes }} 分钟后提醒
      </button>
    </div>
    <button class="text-button reminder-settings" @click="run('open_settings')">
      调整提醒设置 <Icon name="arrow" :size="14" />
    </button>
  </main>
</template>
