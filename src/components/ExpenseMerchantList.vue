<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { CATEGORY_DOT_COLOR, CATEGORY_LABELS, type Category } from '../lib/transactions/constants'
import { useReveal } from '../composables/useReveal'
import type { MerchantGroup } from '../lib/expenses/merchants'

// Only the categories a merchant list can actually contain — savings/contributions
// never appear here (excluded upstream in groupByMerchant).
const LEGEND_CATEGORIES = ['fixed', 'discretionary', 'uncategorized'] as const

const props = defineProps<{
  merchants: MerchantGroup[]
}>()

const emit = defineEmits<{
  select: [searchTerm: string]
}>()

const { progress: reveal, play } = useReveal(500)
onMounted(play)
watch(() => props.merchants, play)

const VISIBLE_COUNT = 8
const expanded = ref(false)
watch(() => props.merchants, () => { expanded.value = false })

// All categories start enabled; toggling the legend hides a category's merchants
// entirely rather than just dimming them, so the bar baseline re-scales to
// whatever's left visible. Kept across period changes since it's a viewing
// preference, not tied to any one period's data.
const activeCategories = ref<Set<Category>>(new Set(LEGEND_CATEGORIES))
function toggleCategory(c: Category) {
  const next = new Set(activeCategories.value)
  if (next.has(c)) next.delete(c)
  else next.add(c)
  activeCategories.value = next
}

const filteredMerchants = computed(() => props.merchants.filter((m) => activeCategories.value.has(m.category)))

const visible = computed(() => expanded.value ? filteredMerchants.value : filteredMerchants.value.slice(0, VISIBLE_COUNT))
const overflow = computed(() => filteredMerchants.value.slice(VISIBLE_COUNT))
const overflowTotal = computed(() => overflow.value.reduce((sum, m) => sum + m.total, 0))

// Bars scale against the largest visible merchant (not each one's share of total
// spend), so the biggest line actually reaches full width instead of everything
// reading small relative to a long tail of minor purchases. Scaled off the full
// filtered list — not just what's visible — so proportions don't jump when
// expanding, and re-baselines automatically when a category is toggled off.
const maxTotal = computed(() => filteredMerchants.value[0]?.total ?? 0)
function barWidth(m: MerchantGroup): number {
  return maxTotal.value > 0 ? (m.total / maxTotal.value) * 100 : 0
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

function onSelect(m: MerchantGroup) {
  if (!m.searchTerm) return
  emit('select', m.searchTerm)
}

// Only shown for categories actually present, so a period with no uncategorized
// spend doesn't carry a dangling "Uncategorized" swatch nobody needs.
const presentCategories = computed(() => {
  const present = new Set(props.merchants.map((m) => m.category))
  return LEGEND_CATEGORIES.filter((c) => present.has(c))
})
</script>

<template>
  <div v-if="merchants.length === 0" class="flex flex-col items-center gap-2 py-8 text-center">
    <UIcon name="i-ph-storefront" class="size-6 text-dimmed" />
    <p class="text-sm text-muted">Nothing recorded for this period yet.</p>
  </div>
  <div v-else class="space-y-4">
    <div v-if="presentCategories.length > 1" class="flex flex-wrap gap-x-4 gap-y-1">
      <button
        v-for="c in presentCategories"
        :key="c"
        type="button"
        class="flex items-center gap-1.5 text-xs rounded-full px-1.5 py-0.5 -mx-1.5 -my-0.5 transition-opacity"
        :class="activeCategories.has(c) ? 'text-muted hover:opacity-75' : 'text-dimmed opacity-40 hover:opacity-60'"
        @click="toggleCategory(c)"
      >
        <span class="size-2 rounded-full shrink-0" :class="CATEGORY_DOT_COLOR[c]" />
        {{ CATEGORY_LABELS[c] }}
      </button>
    </div>

    <div v-if="filteredMerchants.length === 0" class="flex flex-col items-center gap-2 py-8 text-center">
      <UIcon name="i-ph-eye-slash" class="size-6 text-dimmed" />
      <p class="text-sm text-muted">All categories are hidden — toggle one on above.</p>
    </div>
    <div v-else class="space-y-3">
      <div
        v-for="m in visible"
        :key="m.key"
        class="group"
        :class="m.searchTerm ? 'cursor-pointer' : ''"
        :role="m.searchTerm ? 'button' : undefined"
        :tabindex="m.searchTerm ? 0 : -1"
        @click="onSelect(m)"
        @keydown.enter="onSelect(m)"
      >
        <div class="flex items-center justify-between gap-3 text-sm mb-1">
          <span class="flex items-center gap-2 min-w-0">
            <span class="size-2 rounded-full shrink-0" :class="CATEGORY_DOT_COLOR[m.category]" />
            <span
              class="truncate text-heading"
              :class="m.searchTerm ? 'group-hover:underline' : ''"
            >{{ m.displayName }}</span>
            <span class="text-xs text-dimmed shrink-0 tabular-nums">{{ m.count }}×</span>
          </span>
          <span class="font-mono tabular-nums text-heading shrink-0">{{ money(m.total * reveal) }}</span>
        </div>
        <div class="h-1.5 rounded-full bg-muted/15 overflow-hidden">
          <div
            class="h-full rounded-full"
            :class="CATEGORY_DOT_COLOR[m.category]"
            :style="{ width: (barWidth(m) * reveal) + '%' }"
          />
        </div>
      </div>

      <div v-if="overflow.length > 0 || expanded" class="flex items-center justify-between pt-1">
        <button
          type="button"
          class="flex items-center gap-1.5 text-xs text-muted hover:text-heading"
          @click="expanded = !expanded"
        >
          <UIcon :name="expanded ? 'i-ph-caret-up' : 'i-ph-caret-down'" class="size-3.5" />
          {{ expanded ? 'Show fewer merchants' : `Show ${overflow.length} more merchant${overflow.length === 1 ? '' : 's'}` }}
        </button>
        <span v-if="!expanded" class="font-mono tabular-nums text-xs text-muted">{{ money(overflowTotal * reveal) }}</span>
      </div>
    </div>
  </div>
</template>
