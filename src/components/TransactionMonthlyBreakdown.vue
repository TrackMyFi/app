<script setup lang="ts">
import { computed } from 'vue'
import { CATEGORY_LABELS } from '../lib/transactions/constants'
import { classifyFlow } from '../lib/transactions/flow'
import type { Transaction } from '../lib/types/Transaction'
import type { Account } from '../lib/types/Account'

const props = defineProps<{
  transactions: Transaction[]
  accounts: Account[]
}>()

const CATEGORY_ORDER = ['savings', 'fixed', 'discretionary', 'uncategorized'] as const

const CATEGORY_BAR_COLOR: Record<string, string> = {
  savings:       'bg-info/60',
  fixed:         'bg-warning/60',
  discretionary: 'bg-error/60',
  uncategorized: 'bg-inverted/30',
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

  return { income, byCategory }
})

const categoryRows = computed(() =>
  CATEGORY_ORDER
    .filter(cat => totals.value.byCategory.has(cat))
    .map(cat => ({
      key: cat,
      label: CATEGORY_LABELS[cat],
      amount: totals.value.byCategory.get(cat)!,
      pct: totals.value.income > 0
        ? (totals.value.byCategory.get(cat)! / totals.value.income) * 100
        : 0,
    }))
)

function money(n: number) {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}
</script>

<template>
  <div class="space-y-2.5 py-1">
    <!-- Income -->
    <div class="flex items-center gap-3">
      <span class="w-28 text-xs text-muted shrink-0">Income</span>
      <div class="flex-1 h-4 bg-muted/20 rounded-full overflow-hidden">
        <div class="h-full bg-success/60 rounded-full" style="width:100%" />
      </div>
      <span class="w-24 text-right text-sm font-semibold tabular-nums text-success">{{ money(totals.income) }}</span>
      <span class="w-8 text-right text-xs text-muted">100%</span>
    </div>

    <div class="border-t border-default" />

    <!-- Expense categories -->
    <div v-for="row in categoryRows" :key="row.key" class="flex items-center gap-3">
      <span class="w-28 text-xs text-muted shrink-0">{{ row.label }}</span>
      <div class="flex-1 h-4 bg-muted/20 rounded-full overflow-hidden">
        <div
          class="h-full rounded-full transition-all"
          :class="CATEGORY_BAR_COLOR[row.key] ?? 'bg-gray-400'"
          :style="{ width: Math.min(row.pct, 100) + '%' }"
        />
      </div>
      <span class="w-24 text-right text-sm tabular-nums">{{ money(row.amount) }}</span>
      <span class="w-8 text-right text-xs text-muted">{{ row.pct.toFixed(0) }}%</span>
    </div>
  </div>
</template>
