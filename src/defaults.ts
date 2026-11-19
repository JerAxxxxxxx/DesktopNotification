import type { Settings } from "./types";

export const defaultSettings: Settings = {
  startTime: "18:00",
  endTime: "21:00",
  intervalMinutes: 10,
  days: [true, true, true, true, true, false, false],
  message: "今天也辛苦了，收拾一下，准备下班吧。",
  sound: true,
  autostart: false,
};
