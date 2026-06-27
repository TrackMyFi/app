<script setup lang="ts">
defineProps<{
  label: string
  value: string
  hint?: string
  hero?: boolean
  color?: 'default' | 'success'
  trend?: { text: string; positive: boolean } | null
}>()
</script>

<template>
  <UCard class="h-full">
    <div class="text-sm text-muted">{{ label }}</div>
    <div :class="['font-semibold font-mono mt-1', hero ? 'text-3xl' : 'text-2xl', color === 'success' ? 'text-success' : '']">{{ value }}</div>
    <div v-if="hint" class="text-xs text-muted mt-1">{{ hint }}</div>
    <div v-if="trend" class="text-xs font-mono tabular-nums mt-1" :class="trend.positive ? 'text-success' : 'text-error'">{{ trend.text }}</div>
  </UCard>
</template>
