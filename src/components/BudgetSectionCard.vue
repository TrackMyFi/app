<script setup lang="ts">
import { computed } from 'vue'
import { useBudgetStore } from '../stores/budget'
import { useAccountsStore } from '../stores/accounts'
import type { Transaction } from '../lib/types/Transaction'

const props = defineProps<{
  section: 'income' | 'savings' | 'fixed' | 'discretionary'
  transactions: Transaction[]
  /** ISO date of the selected month's first day — rows dated earlier were carried in */
  monthStart: string
  carriedFromLabel: string
}>()

const store = useBudgetStore()
const accountsStore = useAccountsStore()

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

function accountName(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

// A row dated before the selected month was carried in by the paycheck
// attribution preference — badge it so the numbers stay explainable.
function isCarriedIn(t: { date: string }): boolean {
  return t.date < props.monthStart
}

const emptyMessage = computed(() => {
  switch (props.section) {
    case 'income': return 'No income transactions this month.'
    case 'savings': return 'No contributions this month.'
    case 'fixed': return 'No bills this month.'
    case 'discretionary': return 'No spending this month.'
  }
})

// Fixed widths on everything but Description so the columns line up across
// section cards for vertical scanning; the lone Amount column spans the same
// room as Gross + Net so the Account column and right edge stay aligned.
const incomeColumns = [
  { accessorKey: 'date', header: 'Date', meta: { class: { th: 'w-28' } } },
  { accessorKey: 'description', header: 'Description' },
  { id: 'account', header: 'Account', meta: { class: { th: 'w-56' } } },
  { id: 'gross', header: 'Gross', meta: { class: { th: 'w-32 text-right', td: 'text-right tabular-nums' } } },
  { id: 'net', header: 'Net', meta: { class: { th: 'w-32 text-right', td: 'text-right tabular-nums' } } },
]

const sectionColumns = [
  { accessorKey: 'date', header: 'Date', meta: { class: { th: 'w-28' } } },
  { accessorKey: 'description', header: 'Description' },
  { id: 'account', header: 'Account', meta: { class: { th: 'w-56' } } },
  { id: 'amount', header: 'Amount', meta: { class: { th: 'w-64 text-right', td: 'text-right tabular-nums' } } },
]
</script>

<template>
  <div class="border border-default rounded-lg overflow-hidden">
    <div class="bg-elevated px-4 py-1 border-b border-default">
      <span class="text-xs font-medium uppercase tracking-wide">
        {{ section }} — {{ transactions.length }} transaction{{ transactions.length === 1 ? '' : 's' }}
      </span>
    </div>

    <!-- Income table: Gross + Net columns -->
    <UTable v-if="section === 'income'" :data="transactions" :columns="incomeColumns" :empty="emptyMessage">
      <template #description-cell="{ row }">
        <span class="inline-flex items-center gap-2">
          {{ row.original.description }}
          <UBadge
            v-if="isCarriedIn(row.original)"
            color="info"
            variant="subtle"
            size="sm"
            icon="i-ph-arrow-bend-down-right"
            title="Month-end paycheck counted toward the month it funds"
          >from {{ carriedFromLabel }}</UBadge>
        </span>
      </template>
      <template #account-cell="{ row }">
        <span class="text-muted">{{ accountName(row.original.accountId) }}</span>
      </template>
      <template #gross-cell="{ row }">
        {{ money(row.original.paycheckId != null ? (store.paycheckGrossMap[row.original.paycheckId] ?? row.original.amount) : row.original.amount) }}
      </template>
      <template #net-cell="{ row }">{{ money(row.original.amount) }}</template>
    </UTable>

    <!-- All other sections: single Amount column -->
    <UTable v-else :data="transactions" :columns="sectionColumns" :empty="emptyMessage">
      <template #description-cell="{ row }">
        <span class="inline-flex items-center gap-2">
          {{ row.original.description }}
          <UBadge
            v-if="isCarriedIn(row.original)"
            color="info"
            variant="subtle"
            size="sm"
            icon="i-ph-arrow-bend-down-right"
            title="Month-end paycheck counted toward the month it funds"
          >from {{ carriedFromLabel }}</UBadge>
        </span>
      </template>
      <template #account-cell="{ row }">
        <span class="text-muted">{{ accountName(row.original.accountId) }}</span>
      </template>
      <template #amount-cell="{ row }">{{ money(row.original.amount) }}</template>
    </UTable>
  </div>
</template>
