<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { useBudgetStore } from '../stores/budget'
import { useAccountsStore } from '../stores/accounts'
import CurrencyInput from '../components/CurrencyInput.vue'

const store = useBudgetStore()
const accountsStore = useAccountsStore()
const toast = useToast()

const editingTarget = ref(false)
const targetInput = ref<number | null>(null)

const monthItems = computed(() =>
  store.months.map((m) => ({
    label: formatMonth(m.year, m.month),
    value: `${m.year}-${m.month}`,
  }))
)

const selectedMonthKey = ref<string>('')

function formatMonth(year: number, month: number): string {
  return DateTime.fromObject({ year, month }).toFormat('MMMM yyyy')
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}


function accountName(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

async function onMonthChange(value: unknown) {
  const key = value as string
  selectedMonthKey.value = key
  const [year, month] = key.split('-').map(Number)
  await store.load(year, month)
}

function setActiveSection(section: 'income' | 'savings' | 'fixed' | 'discretionary') {
  store.activeSection = section
}

function openTargetEdit() {
  targetInput.value = store.target?.savingsTarget ?? null
  editingTarget.value = true
}

function cancelTargetEdit() {
  editingTarget.value = false
  targetInput.value = null
}

async function saveTarget() {
  if (targetInput.value !== null && targetInput.value >= 0) {
    await store.setTarget(targetInput.value)
    toast.add({ title: 'Budget target saved', color: 'success' })
  }
  editingTarget.value = false
  targetInput.value = null
}

const detailTransactions = computed(() => {
  if (!store.summary || !store.activeSection) return []
  return store.summary[store.activeSection].transactions
})

const emptyMessage = computed(() => {
  switch (store.activeSection) {
    case 'income': return 'No income transactions this month.'
    case 'savings': return 'No contributions this month.'
    case 'fixed': return 'No bills this month.'
    case 'discretionary': return 'No spending this month.'
    default: return 'No transactions this month.'
  }
})

const savingsSubLabel = computed(() => {
  if (editingTarget.value) return null
  if (!store.target) return 'set target'
  if (store.target.isInherited) {
    const from = formatMonth(store.target.sourceYear, store.target.sourceMonth)
    return `from ${from}`
  }
  return `target ${money(store.target.savingsTarget)}`
})


const discretionaryRemaining = computed(() => {
  if (!store.summary) return 0
  return store.summary.freeMoneyRemaining
})

const incomeColumns = [
  { accessorKey: 'date', header: 'Date' },
  { accessorKey: 'description', header: 'Description' },
  { id: 'account', header: 'Account' },
  { id: 'gross', header: 'Gross', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'net', header: 'Net', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
]

const sectionColumns = [
  { accessorKey: 'date', header: 'Date' },
  { accessorKey: 'description', header: 'Description' },
  { id: 'account', header: 'Account' },
  { id: 'amount', header: 'Amount', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
]

onMounted(async () => {
  await Promise.all([accountsStore.load(), store.loadMonths()])
  if (store.months.length > 0) {
    const m = store.months[0]
    selectedMonthKey.value = `${m.year}-${m.month}`
    await store.load(m.year, m.month)
  }
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Budget</h1>
      <USelect
        :model-value="selectedMonthKey"
        :items="monthItems"
        class="w-44"
        @update:model-value="onMonthChange"
      />
    </div>

    <!-- Empty state: no months at all -->
    <p v-if="!store.months.length" class="text-muted text-sm">
      No transaction data yet.
    </p>

    <!-- Formula row + detail panel -->
    <template v-else-if="store.summary">
      <!-- Formula columns -->
      <div class="flex border border-default rounded-lg">
        <!-- Income -->
        <button
          class="relative flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors flex-1 min-w-0 rounded-l-lg border-r border-default"
          :class="store.activeSection === 'income' ? 'bg-elevated' : ''"
          @click="setActiveSection('income')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Income</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.grossIncome) }}</span>
          <span class="text-xs text-muted">Net: {{ money(store.summary.netIncome) }}</span>
          <span class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 z-10 size-6 rounded-full border border-default flex items-center justify-center text-xs text-muted select-none pointer-events-none" style="background: var(--ui-bg)">−</span>
        </button>

        <!-- Savings -->
        <button
          class="relative flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors flex-1 min-w-0 border-r border-default"
          :class="store.activeSection === 'savings' ? 'bg-elevated' : ''"
          @click="setActiveSection('savings')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Savings</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.savings.total) }}</span>

          <!-- Inline target edit -->
          <template v-if="editingTarget">
            <div class="flex items-center gap-1 mt-1" @click.stop>
              <CurrencyInput
                v-model="targetInput"
                size="xs"
                class="w-24"
                @keyup.enter="saveTarget"
                @keyup.escape="cancelTargetEdit"
              />
              <UButton size="xs" @click="saveTarget">Save</UButton>
            </div>
          </template>
          <template v-else>
            <span class="text-xs flex items-center gap-1 text-muted">
              {{ savingsSubLabel }}
              <button
                class="ml-1 opacity-50 hover:opacity-100 transition-opacity"
                title="Edit savings target"
                @click.stop="openTargetEdit"
              >
                <span class="i-ph-pencil w-3 h-3 inline-block" />
              </button>
            </span>
          </template>
          <span class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 z-10 size-6 rounded-full border border-default flex items-center justify-center text-xs text-muted select-none pointer-events-none" style="background: var(--ui-bg)">−</span>
        </button>

        <!-- Taxes (non-clickable) -->
        <div class="relative flex flex-col gap-1 p-4 flex-1 min-w-0 border-r border-default">
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Taxes</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.taxes) }}</span>
          <span class="text-xs text-muted">withheld</span>
          <span class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 z-10 size-6 rounded-full border border-default flex items-center justify-center text-xs text-muted select-none pointer-events-none" style="background: var(--ui-bg)">−</span>
        </div>

        <!-- Fixed -->
        <button
          class="relative flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors flex-1 min-w-0 border-r border-default"
          :class="store.activeSection === 'fixed' ? 'bg-elevated' : ''"
          @click="setActiveSection('fixed')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Fixed</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.fixed.total) }}</span>
          <span class="text-xs text-muted">
            {{ store.summary.fixed.transactions.length }} transaction{{ store.summary.fixed.transactions.length === 1 ? '' : 's' }}
          </span>
          <span class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 z-10 size-6 rounded-full border border-default flex items-center justify-center text-xs text-muted select-none pointer-events-none" style="background: var(--ui-bg)">=</span>
        </button>

        <!-- Free Money (non-clickable, green bg) -->
        <div class="flex flex-col gap-1 p-4 border-r border-default bg-green-500/10 flex-1 min-w-0">
          <span class="text-xs font-semibold uppercase tracking-wide text-green-700 dark:text-green-400">Free Money</span>
          <span class="text-xl font-bold tabular-nums text-green-700 dark:text-green-400">{{ money(store.summary.freeMoney) }}</span>
          <span class="text-xs text-green-600/70 dark:text-green-500/70">&nbsp;</span>
        </div>

        <!-- Discretionary -->
        <button
          class="flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors flex-1 min-w-0 rounded-r-lg"
          :class="store.activeSection === 'discretionary' ? 'bg-elevated' : ''"
          @click="setActiveSection('discretionary')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Discretionary</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.discretionary.total) }}</span>
          <span
            class="text-xs font-medium"
            :class="discretionaryRemaining >= 0 ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'"
          >
            {{ money(discretionaryRemaining) }} remaining
          </span>
        </button>
      </div>

      <!-- Detail panel -->
      <div class="border border-default rounded-lg overflow-hidden">
        <div class="bg-elevated px-4 py-2 border-b border-default">
          <span v-if="store.activeSection === 'income'" class="text-sm font-medium uppercase tracking-wide">
            INCOME — {{ store.summary.income.transactions.length }} transaction{{ store.summary.income.transactions.length === 1 ? '' : 's' }}
          </span>
          <span v-else class="text-sm font-medium capitalize">{{ store.activeSection }}</span>
        </div>
        <!-- Income table: Gross + Net columns -->
        <UTable v-if="store.activeSection === 'income'" :data="detailTransactions" :columns="incomeColumns" :empty="emptyMessage">
          <template #account-cell="{ row }">
            <span class="text-muted">{{ accountName(row.original.accountId) }}</span>
          </template>
          <template #gross-cell="{ row }">
            {{ money(row.original.paycheckId != null ? (store.paycheckGrossMap[row.original.paycheckId] ?? row.original.amount) : row.original.amount) }}
          </template>
          <template #net-cell="{ row }">{{ money(row.original.amount) }}</template>
        </UTable>

        <!-- All other sections: single Amount column -->
        <UTable v-else :data="detailTransactions" :columns="sectionColumns" :empty="emptyMessage">
          <template #account-cell="{ row }">
            <span class="text-muted">{{ accountName(row.original.accountId) }}</span>
          </template>
          <template #amount-cell="{ row }">{{ money(row.original.amount) }}</template>
        </UTable>
      </div>
    </template>
  </div>
</template>
