<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { CATEGORY_LABELS, CATEGORY_DOT_COLOR } from '../lib/transactions/constants'
import { classifyFlow } from '../lib/transactions/flow'
import { useReveal } from '../composables/useReveal'
import type { Transaction } from '../lib/types/Transaction'
import type { Account } from '../lib/types/Account'

const props = defineProps<{
  transactions: Transaction[]
  accounts: Account[]
}>()

// The bar fills out from zero when a period's data lands, so the allocation
// reads as income "filling up" with where it went rather than snapping in.
const { progress: reveal, play } = useReveal(650)
onMounted(play)
watch(() => props.transactions, play)

// Income is allocated in priority order: what you paid yourself first
// (contributions), then obligations, then discretionary, then the unsorted tail.
const CATEGORY_ORDER = ['savings', 'fixed', 'discretionary', 'uncategorized'] as const

const SEGMENT_COLOR: Record<string, string> = {
  savings:       'bg-info',
  fixed:         'bg-warning',
  discretionary: 'bg-error',
  uncategorized: 'bg-inverted/35',
}

const totals = computed(() => {
  let income = 0
  const byCategory = new Map<string, number>()

  for (const t of props.transactions) {
    const f = classifyFlow(t, props.accounts)
    income += f.inflow
    if (f.outflow > 0 && f.bucket) {
      byCategory.set(f.bucket, (byCategory.get(f.bucket) ?? 0) + f.outflow)
    }
  }

  let outgo = 0
  for (const v of byCategory.values()) outgo += v
  return { income, byCategory, outgo }
})

const hasIncome = computed(() => totals.value.income > 0)
const surplus = computed(() => totals.value.income - totals.value.outgo)
const isDeficit = computed(() => hasIncome.value && totals.value.outgo > totals.value.income)

// Legend shares are against income — the metric that matters. The bar itself is
// scaled to whichever is larger (income or outgo) so every segment stays fully
// visible: in a deficit the segments fill the whole track and the income line
// marks where earnings ran out; the coloured zone past it is the overspend.
const legendBase = computed(() =>
  totals.value.income > 0 ? totals.value.income : totals.value.outgo
)
const barScale = computed(() =>
  Math.max(totals.value.income, totals.value.outgo) || 1
)

const segments = computed(() =>
  CATEGORY_ORDER
    .filter(cat => totals.value.byCategory.has(cat))
    .map(cat => {
      const amount = totals.value.byCategory.get(cat)!
      const pct = legendBase.value > 0 ? (amount / legendBase.value) * 100 : 0
      const barPct = (amount / barScale.value) * 100
      return { key: cat, label: CATEGORY_LABELS[cat], amount, pct, barPct }
    })
)

// Position of the income reference line within the bar (right edge when income
// covers everything, inset when the period overran it).
const incomeLinePct = computed(() => (totals.value.income / barScale.value) * 100)

// Share of income the period consumed — drives the caption verdict.
const usedPct = computed(() =>
  hasIncome.value ? (totals.value.outgo / totals.value.income) * 100 : 0
)

// Unspent track trailing the segments in a surplus month, shrinking as the
// reveal fills the bar. Zero in a deficit, where segments fill the whole track.
const surplusWidth = computed(() => {
  if (isDeficit.value || !hasIncome.value) return 0
  const used = segments.value.reduce((s, seg) => s + seg.barPct, 0) * reveal.value
  return Math.max(0, 100 - used)
})

function money(n: number) {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}
function pctText(n: number) {
  return (n * reveal.value).toFixed(0) + '%'
}
</script>

<template>
  <div class="space-y-3">
    <!-- Allocation bar: where income went, each bucket left-to-right. A surplus
         month trails a faint unspent track; a deficit fills the track and the
         income line marks where earnings ran out — the tinted zone past it is
         the overspend. -->
    <div class="relative h-3 rounded-full bg-muted/15 overflow-hidden flex">
      <div
        v-for="seg in segments"
        :key="seg.key"
        class="h-full shrink-0 first:rounded-l-full"
        :class="SEGMENT_COLOR[seg.key] ?? 'bg-muted'"
        :style="{ width: seg.barPct * reveal + '%' }"
      />
      <div
        v-if="surplusWidth > 0"
        class="h-full shrink-0 rounded-r-full bg-success/15"
        :style="{ width: surplusWidth + '%' }"
      />
      <!-- Overspend zone: from the income line to the bar's end -->
      <div
        v-if="isDeficit"
        class="absolute inset-y-0 right-0 bg-error/20 border-l-2 border-error/70"
        :style="{ left: incomeLinePct + '%' }"
      />
    </div>

    <!-- Caption: surplus / deficit verdict for the bar, kept calm — the stat
         strip above already carries the colour judgment. -->
    <p v-if="hasIncome" class="text-xs text-muted tabular-nums">
      <template v-if="surplus >= 0">
        <span class="text-success">{{ pctText(usedPct) }}</span> of income used
        · <span class="text-heading font-medium">{{ money(surplus * reveal) }}</span> left to spare
      </template>
      <template v-else>
        <span class="text-error">{{ pctText(usedPct) }}</span> of income used
        · <span class="text-heading font-medium">{{ money(-surplus * reveal) }}</span> beyond income
      </template>
    </p>

    <!-- Legend: each bucket's share, scannable left-to-right. -->
    <div class="flex flex-wrap gap-x-6 gap-y-2 pt-1">
      <div v-for="seg in segments" :key="seg.key" class="flex items-baseline gap-2">
        <span class="size-2 rounded-full shrink-0 self-center" :class="CATEGORY_DOT_COLOR[seg.key] ?? 'bg-muted'" />
        <span class="text-xs text-muted">{{ seg.label }}</span>
        <span class="text-sm font-medium tabular-nums text-heading">{{ money(seg.amount * reveal) }}</span>
        <span class="text-xs text-dimmed tabular-nums">{{ pctText(seg.pct) }}</span>
      </div>
    </div>
  </div>
</template>
