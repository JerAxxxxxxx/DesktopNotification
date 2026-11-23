export interface Settings {
  startTime: string;
  endTime: string;
  intervalMinutes: number;
  sedentaryMinutes: number;
  days: boolean[];
  message: string;
  sound: boolean;
  autostart: boolean;
}
export interface Snapshot {
  settings: Settings;
  countdownVisible: boolean;
  paused: boolean;
  phase: "scheduled" | "paused" | "dayOff" | "finished" | "due";
  nextAt: number | null;
  warning: string | null;
  configPath: string;
}
