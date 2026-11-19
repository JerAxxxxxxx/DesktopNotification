export const holidayYear = 2027;

interface Holiday {
  name: string;
  start: string;
  end: string;
}

const holidays: readonly Holiday[] = [
  { name: "元旦", start: "2027-01-01", end: "2027-01-03" },
  { name: "春节", start: "2027-02-05", end: "2027-02-13" },
  { name: "清明节", start: "2027-04-03", end: "2027-04-05" },
  { name: "劳动节", start: "2027-05-01", end: "2027-05-05" },
  { name: "端午节", start: "2027-06-09", end: "2027-06-09" },
  { name: "中秋节", start: "2027-09-15", end: "2027-09-15" },
  { name: "国庆节", start: "2027-10-01", end: "2027-10-07" },
];

const dayMs = 86_400_000;
const beijingOffsetMs = 8 * 60 * 60 * 1000;
const dayNumber = (date: string) => Date.parse(date) / dayMs;

const calendar = holidays.map((holiday) => {
  const first = dayNumber(holiday.start);
  const last = dayNumber(holiday.end);
  return { holiday, first, last, duration: last - first + 1 };
});

export function getHolidayCountdowns(now: number) {
  const today = Math.floor((now + beijingOffsetMs) / dayMs);
  return calendar.map(({ holiday, first, last, duration }) => {
    const status = today < first ? "upcoming" : today <= last ? "ongoing" : "past";
    return {
      ...holiday,
      duration,
      status,
      daysUntil: status === "past" ? null : Math.max(0, first - today),
    };
  });
}
