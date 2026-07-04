<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  accessibleLabel: string
  deferredLabel: string
  /** 0..100 share of investable funds that are accessible before 59½. */
  accessiblePct: number
  statusText: string
  statusColor: 'success' | 'warning' | 'muted'
  /** Fine-print caveat under the status line; omitted when there's nothing to qualify. */
  caveat?: string
}>()

const barWidth = computed(() => `${Math.min(Math.max(props.accessiblePct, 0), 100)}%`)
const statusClass = computed(() =>
  props.statusColor === 'success' ? 'text-success' : props.statusColor === 'warning' ? 'text-warning' : 'text-muted')
</script>

<template>
  <div class="border border-default rounded-lg p-4">
    <h2 class="font-semibold mb-1">Bridge to 59½</h2>
    <p class="text-xs text-muted mb-3">Early retirement spends from accessible accounts until retirement accounts unlock</p>

    <!-- Accessible vs penalty-locked share of investable funds -->
    <div class="h-2.5 rounded-full bg-elevated overflow-hidden">
      <div class="h-full rounded-full bg-primary" :style="{ width: barWidth }" />
    </div>
    <div class="mt-2 flex justify-between text-xs">
      <span>
        <span class="inline-block size-2 rounded-full bg-primary align-middle mr-1.5" aria-hidden="true" />
        <span class="text-muted">Accessible now</span>
        <span class="font-mono tabular-nums font-medium ml-1.5">{{ accessibleLabel }}</span>
      </span>
      <span>
        <span class="inline-block size-2 rounded-full bg-elevated align-middle mr-1.5" aria-hidden="true" />
        <span class="text-muted">Locked until 59½</span>
        <span class="font-mono tabular-nums font-medium ml-1.5">{{ deferredLabel }}</span>
      </span>
    </div>

    <p class="mt-3 text-sm" :class="statusClass">{{ statusText }}</p>
    <p v-if="caveat" class="mt-1 text-xs text-muted">{{ caveat }}</p>
  </div>
</template>
