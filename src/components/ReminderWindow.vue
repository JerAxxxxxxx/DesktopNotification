<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { defaultSettings } from "../defaults";
import Icon from "./AppIcon.vue";

interface ReminderCard {
  id: number;
  preview: boolean;
  message: string;
  intervalMinutes: number;
}

const card = ref<ReminderCard | null>(null);
const busy = ref(false);
const error = ref("");
let unlisten: UnlistenFn | undefined;

function display(value: ReminderCard) {
  card.value = value;
  error.value = "";
}

async function run(action: "pause" | "snooze" | "dismiss" | "settings") {
  if (busy.value || !card.value) return;
  busy.value = true;
  error.value = "";
  try {
    if (action === "settings") await invoke("open_settings");
    else await invoke("card_action", { id: card.value.id, action });
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

onMounted(async () => {
  if (!isTauri()) return;
  try {
    unlisten = await listen<ReminderCard>("reminder-shown", ({ payload }) => display(payload));
    const initial = await invoke<ReminderCard | null>("get_reminder_card");
    if (initial && !card.value) display(initial);
  } catch (cause) {
    error.value = String(cause);
  }
});

onUnmounted(() => unlisten?.());
</script>

<template>
  <main class="reminder-shell">
    <div class="reminder-top">
      <h1 class="reminder-title"><Icon name="sun" :size="19" /> {{ card?.preview ? '测试提醒' : '下班时间到了' }}</h1>
      <button class="icon-button" aria-label="关闭本次提醒" @click="run('dismiss')">
        <Icon name="close" :size="18" />
      </button>
    </div>
    <p v-if="error" class="reminder-message reminder-error" role="alert">{{ error }}</p>
    <p v-else class="reminder-message" tabindex="0">{{ card?.message ?? defaultSettings.message }}</p>
    <div class="reminder-buttons">
      <button class="primary" :disabled="busy || !card" @click="run('pause')">
        <Icon name="check" :size="17" /> 今天不再提醒
      </button>
      <button class="secondary" :disabled="busy || !card" @click="run('snooze')">
        {{ card?.intervalMinutes ?? defaultSettings.intervalMinutes }} 分钟后提醒
      </button>
    </div>
    <div class="reminder-footer">
      <span v-if="card?.preview">预览操作仅关闭此卡片</span>
      <button class="text-button reminder-settings" @click="run('settings')">
        打开设置 <Icon name="arrow" :size="13" />
      </button>
    </div>
  </main>
</template>
