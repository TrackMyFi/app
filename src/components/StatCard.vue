<script setup lang="ts">
defineProps<{ label: string; value: string; hint?: string; hero?: boolean; color?: 'default' | 'success' }>()
</script>

<template>
  <UCard>
    <div class="text-sm text-muted">{{ label }}</div>
    <div :class="['font-semibold font-mono mt-1', hero ? 'text-3xl' : 'text-2xl', color === 'success' ? 'text-success' : '']">{{ value }}</div>
    <div v-if="hint" class="text-xs text-muted mt-1">{{ hint }}</div>
  </UCard>
</template>
