<script setup lang="ts">
defineProps<{ label: string; value: string; hint?: string }>()
</script>

<template>
  <UCard>
    <div class="text-sm text-muted">{{ label }}</div>
    <div class="text-2xl font-semibold mt-1">{{ value }}</div>
    <div v-if="hint" class="text-xs text-muted mt-1">{{ hint }}</div>
  </UCard>
</template>
