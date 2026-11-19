import assert from "node:assert/strict";
import test from "node:test";
import { getHolidayCountdowns } from "../src/holidays.ts";

const at = (value) => new Date(value).getTime();

test("元旦倒计时按北京时间跨年，与电脑时区或当前秒数无关", () => {
  const before = getHolidayCountdowns(at("2026-12-31T15:59:59Z"))[0];
  assert.equal(before.name, "元旦");
  assert.deepEqual([before.status, before.daysUntil], ["upcoming", 1]);
  const on = getHolidayCountdowns(at("2026-12-31T16:00:00Z"))[0];
  assert.deepEqual([on.status, on.daysUntil], ["ongoing", 0]);
});

test("春节倒数到放假首日，假期末日仍显示放假中", () => {
  const spring = (value) => getHolidayCountdowns(at(value)).find((item) => item.name === "春节");
  const before = spring("2027-02-04T23:30:00+08:00");
  assert.deepEqual([before.status, before.daysUntil, before.duration], ["upcoming", 1, 9]);
  assert.equal(spring("2027-02-13T23:59:59+08:00").status, "ongoing");
  assert.equal(spring("2027-02-14T00:00:00+08:00").status, "past");
});

test("已有年份数据到期后不会拿旧日期计算下一年倒计时", () => {
  const rows = getHolidayCountdowns(at("2028-01-01T00:00:00+08:00"));
  assert.equal(rows.length, 7);
  assert.ok(rows.every((item) => item.status === "past" && item.daysUntil === null));
});
