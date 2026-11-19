import { createApp } from "vue";
import CountdownWindow from "./components/CountdownWindow.vue";
import ReminderWindow from "./components/ReminderWindow.vue";
import SettingsWindow from "./components/SettingsWindow.vue";
import "./style.css";

const view = new URLSearchParams(location.search).get("view");
const windows = {
  countdown: CountdownWindow,
  reminder: ReminderWindow,
};

if (view === "countdown" || view === "reminder") {
  document.documentElement.classList.add(`${view}-mode`);
  createApp(windows[view]).mount("#app");
} else {
  createApp(SettingsWindow).mount("#app");
}
