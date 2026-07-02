<script setup lang="ts">
import type { OpportunityItem } from '../lib/expenses/opportunities'

defineProps<{
  items: OpportunityItem[]
}>()

const emit = defineEmits<{
  select: [searchTerm: string]
}>()

const TONE_CLASSES: Record<OpportunityItem['tone'], { swatch: string; icon: string }> = {
  warning: { swatch: 'bg-warning/10', icon: 'text-warning' },
  error: { swatch: 'bg-error/10', icon: 'text-error' },
}
</script>

<template>
  <div v-if="items.length === 0" class="flex flex-col items-center gap-2 py-8 text-center">
    <UIcon name="i-ph-check-circle" class="size-6 text-dimmed" />
    <p class="text-sm text-muted">Nothing to flag right now — spend looks steady.</p>
  </div>
  <ul v-else class="divide-y divide-default">
    <li
      v-for="item in items"
      :key="item.id"
      class="flex items-center gap-3 py-3"
      :class="item.searchTerm ? 'cursor-pointer group' : ''"
      :role="item.searchTerm ? 'button' : undefined"
      :tabindex="item.searchTerm ? 0 : -1"
      @click="item.searchTerm && emit('select', item.searchTerm)"
      @keydown.enter="item.searchTerm && emit('select', item.searchTerm)"
    >
      <span
        class="flex items-center justify-center size-8 rounded-full shrink-0"
        :class="TONE_CLASSES[item.tone].swatch"
      >
        <UIcon :name="item.icon" class="size-4" :class="TONE_CLASSES[item.tone].icon" />
      </span>
      <div class="min-w-0 flex-1">
        <p class="text-sm text-heading" :class="item.searchTerm ? 'group-hover:underline' : ''">{{ item.title }}</p>
        <p class="text-xs text-muted mt-0.5">{{ item.subtitle }}</p>
      </div>
      <span class="font-mono tabular-nums text-sm text-heading shrink-0">{{ item.trailing }}</span>
    </li>
  </ul>
</template>
