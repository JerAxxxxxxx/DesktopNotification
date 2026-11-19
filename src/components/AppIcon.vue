<script setup lang="ts">
defineProps<{ name: string; size?: number }>();
const paths: Record<string, string> = {
  clock: "M12 8v4l3 2 M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0",
  calendar: "M5 5h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z M7 3v4 M17 3v4 M3 10h18 M8 15h2 M14 15h2",
  chevron: "m6 9 6 6 6-6",
  bell: "M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9 M10 21h4",
  arrow: "M5 12h14 M13 6l6 6-6 6",
  check: "M5 12l4 4L19 6",
  pause: "M9 5v14 M15 5v14",
  play: "M8 5l11 7-11 7z",
  sun: "M12 2v2 M12 20v2 M2 12h2 M20 12h2 M5 5l1.5 1.5 M17.5 17.5L19 19 M5 19l1.5-1.5 M17.5 6.5L19 5 M17 12a5 5 0 1 1-10 0 5 5 0 0 1 10 0",
  leaf: "M20 4C10 3 3 7 5 15c3 7 14 5 15-11z M5 20L16 9",
  moon: "M20.5 14.2A8.5 8.5 0 0 1 9.8 3.5 8.5 8.5 0 1 0 20.5 14.2z M17 4v4 M15 6h4 M21 8v3 M19.5 9.5h3",
  close: "M6 6l12 12 M6 18L18 6",
  volume: "M11 5L6 9H3v6h3l5 4z M15 8a6 6 0 0 1 0 8 M18 5a10 10 0 0 1 0 14",
  power: "M12 2v10 M5.6 5.6a9 9 0 1 0 12.8 0",
  info: "M12 11v6 M12 7h.01 M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0",
};
</script>
<template>
  <svg
    :width="size || 20"
    :height="size || 20"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="paths[name] || paths.clock" />
  </svg>
</template>
