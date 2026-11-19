import assert from "node:assert/strict";
import test from "node:test";
import { getWorkCountdown } from "../src/countdown.ts";

const settings = {
  startTime: "18:00",
  days: [true, true, true, true, true, false, false],
};
const local = (day, hour, minute, second = 0, ms = 0) =>
  new Date(2026, 10, day, hour, minute, second, ms).getTime();

test("counts down to the configured local off-work time", () => {
  assert.deepEqual(getWorkCountdown(settings, local(18, 16, 57, 59)), {
    state: "counting",
    value: "01:02:01",
  });
  assert.deepEqual(
    getWorkCountdown({ ...settings, startTime: "17:30" }, local(18, 17, 29, 59, 950)),
    { state: "counting", value: "00:00:01" },
  );
});

test("switches to overtime at the exact off-work time", () => {
  assert.deepEqual(getWorkCountdown(settings, local(18, 18, 0)), {
    state: "overtime",
    value: "00:00:00",
  });
  assert.deepEqual(getWorkCountdown(settings, local(18, 18, 0, 0, 999)), {
    state: "overtime",
    value: "00:00:00",
  });
});

test("keeps counting overtime after reminders end, until the next day", () => {
  assert.deepEqual(
    getWorkCountdown({ ...settings, endTime: "21:00" }, local(18, 23, 59, 23)),
    { state: "overtime", value: "05:59:23" },
  );
  assert.deepEqual(getWorkCountdown(settings, local(19, 0, 0)), {
    state: "counting",
    value: "18:00:00",
  });
});

test("shows day-off when today's weekday is disabled", () => {
  assert.deepEqual(getWorkCountdown(settings, local(21, 9, 0)), { state: "dayOff" });
  assert.deepEqual(getWorkCountdown(settings, local(21, 23, 0)), { state: "dayOff" });
});
