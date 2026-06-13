<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { useBudgetStore } from '../stores/budget'
import { useAccountsStore } from '../stores/accounts'

const store = useBudgetStore()
const accountsStore = useAccountsStore()

const editingTarget = ref(false)
const targetInput = ref<number | string>('')

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

function formatMoney(n: number): string {
  return money(n)
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
  targetInput.value = store.target?.savingsTarget ?? ''
  editingTarget.value = true
}

function cancelTargetEdit() {
  editingTarget.value = false
  targetInput.value = ''
}

async function saveTarget() {
  const val = Number(targetInput.value)
  if (!isNaN(val) && val >= 0) {
    await store.setTarget(val)
  }
  editingTarget.value = false
  targetInput.value = ''
}

const detailTransactions = computed(() => {
  if (!store.summary || !store.activeSection) return []
  return store.summary[store.activeSection].transactions
})

const emptyMessage = computed(() => {
  switch (store.activeSection) {
    case 'income': return 'No income transactions this month.'
    case 'savings': return 'No contributions this month.'
    case 'fixed': return 'No fixed expenses this month.'
    case 'discretionary': return 'No discretionary transactions this month.'
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

const savingsSubLabelMuted = computed(() => !!store.target?.isInherited)

const discretionaryRemaining = computed(() => {
  if (!store.summary) return 0
  return store.summary.freeMoneyRemaining
})

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
      <div class="grid border border-default rounded-lg overflow-hidden" style="grid-template-columns: 1fr auto 1fr auto 1fr auto 1fr auto 1fr 1fr">
        <!-- Income -->
        <button
          class="flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors"
          :class="store.activeSection === 'income' ? 'bg-elevated' : ''"
          @click="setActiveSection('income')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Income</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.grossIncome) }}</span>
          <span class="text-xs text-muted">Net: {{ formatMoney(store.summary.netIncome) }}</span>
        </button>

        <!-- Operator: − -->
        <div class="flex items-center justify-center px-2 text-sm text-muted select-none border-x border-default">−</div>

        <!-- Savings -->
        <button
          class="flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors"
          :class="store.activeSection === 'savings' ? 'bg-elevated' : ''"
          @click="setActiveSection('savings')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Savings</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.savings.total) }}</span>

          <!-- Inline target edit -->
          <template v-if="editingTarget">
            <div class="flex items-center gap-1 mt-1" @click.stop>
              <UInput
                v-model="targetInput"
                type="number"
                size="xs"
                class="w-24"
                placeholder="0"
                @keyup.enter="saveTarget"
                @keyup.escape="cancelTargetEdit"
              />
              <UButton size="xs" @click="saveTarget">Save</UButton>
            </div>
          </template>
          <template v-else>
            <span
              class="text-xs flex items-center gap-1"
              :class="savingsSubLabelMuted ? 'text-muted' : 'text-muted'"
            >
              {{ savingsSubLabel }}
              <button
                class="ml-1 opacity-50 hover:opacity-100 transition-opacity"
                title="Edit savings target"
                @click.stop="openTargetEdit"
              >
                <span class="i-lucide-pencil w-3 h-3 inline-block" />
              </button>
            </span>
          </template>
        </button>

        <!-- Operator: − -->
        <div class="flex items-center justify-center px-2 text-sm text-muted select-none border-x border-default">−</div>

        <!-- Taxes (non-clickable) -->
        <div class="flex flex-col gap-1 p-4">
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Taxes</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.taxes) }}</span>
          <span class="text-xs text-muted">withheld</span>
        </div>

        <!-- Operator: − -->
        <div class="flex items-center justify-center px-2 text-sm text-muted select-none border-x border-default">−</div>

        <!-- Fixed -->
        <button
          class="flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors"
          :class="store.activeSection === 'fixed' ? 'bg-elevated' : ''"
          @click="setActiveSection('fixed')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Fixed</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.fixed.total) }}</span>
          <span class="text-xs text-muted">
            {{ store.summary.fixed.transactions.length }} transaction{{ store.summary.fixed.transactions.length === 1 ? '' : 's' }}
          </span>
        </button>

        <!-- Operator: = -->
        <div class="flex items-center justify-center px-2 text-sm text-muted select-none border-x border-default">=</div>

        <!-- Free Money (non-clickable, green bg) -->
        <div class="flex flex-col gap-1 p-4 border-r border-default bg-green-500/10">
          <span class="text-xs font-semibold uppercase tracking-wide text-green-700 dark:text-green-400">Free Money</span>
          <span class="text-xl font-bold tabular-nums text-green-700 dark:text-green-400">{{ money(store.summary.freeMoney) }}</span>
          <span class="text-xs text-green-600/70 dark:text-green-500/70">&nbsp;</span>
        </div>

        <!-- Discretionary -->
        <button
          class="flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors"
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
        <table class="w-full text-sm">
          <thead class="text-left text-muted border-b border-default">
            <tr>
              <th class="px-4 py-2 font-normal w-28">Date</th>
              <th class="py-2 font-normal">Description</th>
              <th class="py-2 font-normal">Account</th>
              <th class="px-4 py-2 font-normal text-right">Amount</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="t in detailTransactions"
              :key="t.id"
              class="border-t border-default/50"
            >
              <td class="px-4 py-2 text-muted w-28">{{ t.date }}</td>
              <td class="py-2">{{ t.description }}</td>
              <td class="py-2 text-muted">{{ accountName(t.accountId) }}</td>
              <td class="px-4 py-2 text-right tabular-nums">{{ money(t.amount) }}</td>
            </tr>
            <tr v-if="!detailTransactions.length">
              <td colspan="4" class="px-4 py-6 text-center text-muted">{{ emptyMessage }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>
