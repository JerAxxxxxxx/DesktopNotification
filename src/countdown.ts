import type { Settings } from "./types";

type WorkCountdown =
  | { state: "counting" | "overtime"; value: string }
  | { state: "dayOff" };

export function getWorkCountdown(
  settings: Pick<Settings, "startTime" | "days">,
  now: number,
): WorkCountdown {
  const target = new Date(now);
  if (!settings.days[(target.getDay() + 6) % 7]) return { state: "dayOff" };

  const hours = Number(settings.startTime.slice(0, 2));
  const minutes = Number(settings.startTime.slice(3, 5));
  const delta = target.setHours(hours, minutes, 0, 0) - now;
  const state = delta > 0 ? "counting" : "overtime";
  const seconds = delta > 0 ? Math.ceil(delta / 1000) : Math.floor(-delta / 1000);

  const value = [Math.floor(seconds / 3600), Math.floor(seconds / 60) % 60, seconds % 60]
    .map((part) => String(part).padStart(2, "0"))
    .join(":");
  return { state, value };
}
