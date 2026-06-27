<script setup lang="ts">
defineProps<{
  label: string
  value: string
  hint?: string
  hero?: boolean
  color?: 'default' | 'success' | 'error'
  trend?: { text: string; positive: boolean } | null
  trends?: { text: string; positive: boolean }[]
}>()
</script>

<template>
  <UCard class="h-full">
    <div class="text-sm text-muted">{{ label }}</div>
    <div :class="['font-semibold font-mono mt-1', hero ? 'text-3xl' : 'text-2xl', color === 'success' ? 'text-success' : color === 'error' ? 'text-error' : '']">{{ value }}</div>
    <div v-if="hint" class="text-xs text-muted mt-1">{{ hint }}</div>
    <div v-if="trend" class="text-xs font-mono tabular-nums mt-1" :class="trend.positive ? 'text-success' : 'text-error'">{{ trend.text }}</div>
    <template v-if="trends">
      <div v-for="t in trends" :key="t.text" class="text-xs font-mono tabular-nums mt-0.5" :class="t.positive ? 'text-success' : 'text-error'">{{ t.text }}</div>
    </template>
  </UCard>
</template>
