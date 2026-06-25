<script setup lang="ts">
// Hidden SVG holding the zero-threshold gradient. Lives in the document so chart
// paths can reference it via url(#id); rendered once per chart instance.
defineProps<{ id: string; offset: number; positive: string; negative: string }>()
</script>

<template>
  <svg aria-hidden="true" focusable="false" width="0" height="0" style="position:absolute">
    <defs>
      <linearGradient :id="id" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" :stop-color="positive" />
        <stop :offset="offset" :stop-color="positive" />
        <stop :offset="offset" :stop-color="negative" />
        <stop offset="1" :stop-color="negative" />
      </linearGradient>
    </defs>
  </svg>
</template>
