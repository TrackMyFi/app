<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { useTransactionsStore } from '../stores/transactions'
import { useAccountsStore } from '../stores/accounts'
import { transactionTypeItems, categoryItems, labelForTransactionType, labelForCategory } from '../lib/transactions/constants'
import { isLiability } from '../lib/accountTypes'
import * as api from '../lib/api/transactions'
import TransactionForm from '../components/TransactionForm.vue'
import ImportWizard from '../components/ImportWizard.vue'
import TransactionChart from '../components/TransactionChart.vue'
import TransactionMonthlyBreakdown from '../components/TransactionMonthlyBreakdown.vue'
import MonthPicker from '../components/MonthPicker.vue'
import type { Transaction } from '../lib/types/Transaction'
import { confirm } from '@tauri-apps/plugin-dialog'

const store = useTransactionsStore()
const accountsStore = useAccountsStore()

// ─── Modals ───────────────────────────────────────────────────────────────────

const isModalOpen = ref(false)
const editing = ref<Transaction | null>(null)

function openAdd() { editing.value = null; isModalOpen.value = true }
function openEdit(t: Transaction) { editing.value = t; isModalOpen.value = true }
async function onSaved() { isModalOpen.value = false; await loadYearData() }

const isImportOpen = ref(false)
async function onImportDone() { isImportOpen.value = false; await loadYearData() }

async function removeRow(t: Transaction) {
  const ok = await confirm(`Delete "${t.description}"?`, { title: 'Delete transaction' })
  if (ok) { await store.remove(t.id); await loadYearData() }
}

// ─── Month navigation ──────────────────────────────────────────────────────────

const selectedDate = ref(DateTime.now().startOf('month'))
const monthLabel = computed(() => selectedDate.value.toFormat('MMMM yyyy'))
const yearLabel = computed(() => selectedDate.value.toFormat('yyyy'))
const monthStart = computed(() => selectedDate.value.toISODate()!)
const monthEnd = computed(() => selectedDate.value.endOf('month').toISODate()!)

function onMonthChange(dt: DateTime) {
  selectedDate.value = dt
  applyMonthFilter()
}

// ─── Chart mode ────────────────────────────────────────────────────────────────

const chartMode = ref<'monthly' | 'annual'>('monthly')

const chartTitle = computed(() =>
  chartMode.value === 'monthly'
    ? `Expense Breakdown — ${monthLabel.value}`
    : `Income vs. Expense — ${yearLabel.value}`
)

// ─── Secondary filters ─────────────────────────────────────────────────────────

const accountId = ref<number | undefined>(undefined)
const type = ref<string | undefined>(undefined)
const category = ref<string | undefined>(undefined)
const search = ref('')

async function applyFilters() { await applyMonthFilter() }

async function clearFilters() {
  accountId.value = undefined
  type.value = undefined
  category.value = undefined
  search.value = ''
  await applyMonthFilter()
}

// ─── Data loading ──────────────────────────────────────────────────────────────

const yearTransactions = ref<Transaction[]>([])

async function applyMonthFilter() {
  await store.setFilter({
    accountId: accountId.value ?? null,
    type: type.value ?? null,
    category: category.value ?? null,
    search: search.value || null,
    startDate: monthStart.value,
    endDate: monthEnd.value,
    limit: null,
  })
  await loadYearData()
}

async function loadYearData() {
  const result = await api.listTransactions({
    accountId: accountId.value ?? null,
    type: type.value ?? null,
    category: category.value ?? null,
    search: search.value || null,
    startDate: selectedDate.value.startOf('year').toISODate()!,
    endDate: selectedDate.value.endOf('year').toISODate()!,
    limit: null,
  })
  yearTransactions.value = result.rows
}

// ─── Totals ────────────────────────────────────────────────────────────────────

function effectiveDelta(t: Transaction): number {
  if (t.type === 'income') return t.amount
  if (t.type === 'expense') return -t.amount
  if (t.transferAccountId == null) return 0
  const destType = accountsStore.accounts.find(a => a.id === t.transferAccountId)?.type ?? ''
  return isLiability(destType) ? -t.amount : 0
}

function cashFlowTotals(txns: Transaction[]) {
  let income = 0, expense = 0, net = 0
  for (const t of txns) {
    if (t.type === 'income') income += t.amount
    else if (t.type === 'expense') expense += t.amount
    net += effectiveDelta(t)
  }
  return { income, expense, net }
}

const monthlyTotals = computed(() => cashFlowTotals(store.page.rows))
const annualTotals = computed(() => cashFlowTotals(yearTransactions.value))

function savingsRate(totals: { income: number; net: number }): string {
  if (totals.income <= 0) return '—'
  return ((totals.net / totals.income) * 100).toFixed(1) + '%'
}

const monthlySavingsRate = computed(() => savingsRate(monthlyTotals.value))
const annualSavingsRate = computed(() => savingsRate(annualTotals.value))

// ─── Table ─────────────────────────────────────────────────────────────────────

const rows = computed(() => store.page.rows)

const columns = [
  { accessorKey: 'date', header: 'Date' },
  { accessorKey: 'description', header: 'Description', meta: { class: { td: 'max-w-[300px] truncate' } } },
  { id: 'account', header: 'Account' },
  { id: 'type', header: 'Type' },
  { id: 'category', header: 'Category' },
  { id: 'amount', header: 'Amount', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'actions', header: '', meta: { class: { td: 'text-right' } } },
]

function accountName(id: number | null): string {
  if (id == null) return '—'
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

onMounted(async () => {
  await accountsStore.load()
  await applyMonthFilter()
})
</script>

<template>
  <div class="p-6 space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Transactions</h1>
      <div class="flex gap-2">
        <UButton variant="subtle" icon="i-ph-upload" @click="isImportOpen = true">Import CSV</UButton>
        <UButton icon="i-ph-plus" @click="openAdd">Add transaction</UButton>
      </div>
    </div>

    <!-- Chart card -->
    <div class="border border-default rounded-lg p-4">
      <div class="flex items-center justify-between mb-4">
        <MonthPicker :model-value="selectedDate" @update:model-value="onMonthChange" />
        <div class="flex rounded-md overflow-hidden border border-default text-xs">
          <button
            class="px-3 py-1 transition-colors"
            :class="chartMode === 'monthly' ? 'bg-primary text-white' : 'hover:bg-elevated'"
            @click="chartMode = 'monthly'"
          >Monthly</button>
          <button
            class="px-3 py-1 transition-colors border-l border-default"
            :class="chartMode === 'annual' ? 'bg-primary text-white' : 'hover:bg-elevated'"
            @click="chartMode = 'annual'"
          >Annual</button>
        </div>
      </div>
      <p class="text-xs font-semibold uppercase tracking-widest text-muted mb-3">{{ chartTitle }}</p>
      <template v-if="chartMode === 'monthly'">
        <TransactionMonthlyBreakdown
          v-if="store.page.rows.length > 0"
          :transactions="store.page.rows"
          :accounts="accountsStore.accounts"
        />
        <p v-else class="text-sm text-muted text-center py-8">No transactions for this period</p>
      </template>
      <template v-else>
        <TransactionChart
          v-if="yearTransactions.length > 0"
          :transactions="yearTransactions"
          :accounts="accountsStore.accounts"
        />
        <p v-else class="text-sm text-muted text-center py-8">No transactions for this year</p>
      </template>
    </div>

    <!-- Stats row -->
    <div class="grid grid-cols-2 gap-4">
      <div class="border border-default rounded-lg p-4">
        <div class="flex items-center gap-3 mb-2">
          <p class="text-xs font-semibold uppercase tracking-widest text-muted">
            {{ monthLabel }}
          </p>
          <span class="text-xs text-muted/75">
            ({{ store.page.totalCount }} transactions)
          </span>
        </div>
        <div class="flex flex-wrap gap-x-6 gap-y-1 text-sm">
          <span>Income: <strong class="text-success">{{ money(monthlyTotals.income) }}</strong></span>
          <span>Expense: <strong class="text-error">{{ money(monthlyTotals.expense) }}</strong></span>
          <span>Net: <strong>{{ money(monthlyTotals.net) }}</strong></span>
          <span>Savings Rate: <strong :class="monthlyTotals.net >= 0 ? 'text-success' : 'text-error'">{{ monthlySavingsRate }}</strong></span>
        </div>
      </div>
      <div class="border border-default rounded-lg p-4">
        <p class="text-xs font-semibold uppercase tracking-widest text-muted mb-2">{{ yearLabel }} Annual</p>
        <div class="flex flex-wrap gap-x-6 gap-y-1 text-sm">
          <span>Income: <strong class="text-success">{{ money(annualTotals.income) }}</strong></span>
          <span>Expense: <strong class="text-error">{{ money(annualTotals.expense) }}</strong></span>
          <span>Net: <strong>{{ money(annualTotals.net) }}</strong></span>
          <span>Savings Rate: <strong :class="annualTotals.net >= 0 ? 'text-success' : 'text-error'">{{ annualSavingsRate }}</strong></span>
        </div>
      </div>
    </div>

    <!-- Secondary filters -->
    <div class="flex flex-wrap gap-2 items-end">
      <USelect
        v-model="accountId"
        :items="accountsStore.accounts.map((a) => ({ label: a.name, value: a.id }))"
        placeholder="All accounts"
        class="w-44"
      />
      <USelect
        v-model="type"
        :items="transactionTypeItems"
        placeholder="All types"
        class="w-36"
      />
      <USelect
        v-model="category"
        :items="categoryItems"
        placeholder="All categories"
        class="w-40"
      />
      <UInput v-model="search" placeholder="Search description" class="w-52" />
      <UButton @click="applyFilters">Apply</UButton>
      <UButton variant="ghost" @click="clearFilters">Clear</UButton>
    </div>

    <!-- Table -->
    <div class="border border-default rounded-lg overflow-hidden">
      <UTable :data="rows" :columns="columns" empty="No transactions yet.">
        <template #account-cell="{ row }">
          {{ accountName(row.original.accountId) }}
          <span v-if="row.original.type === 'transfer'"> → {{ accountName(row.original.transferAccountId) }}</span>
        </template>
        <template #type-cell="{ row }">{{ labelForTransactionType(row.original.type) }}</template>
        <template #category-cell="{ row }">{{ labelForCategory(row.original.category) }}</template>
        <template #amount-cell="{ row }">{{ money(row.original.amount) }}</template>
        <template #actions-cell="{ row }">
          <UButton size="xs" variant="ghost" icon="i-ph-pencil" @click="openEdit(row.original)" />
          <UButton size="xs" variant="ghost" color="error" icon="i-ph-trash" @click="removeRow(row.original)" />
        </template>
      </UTable>
    </div>

    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit transaction' : 'Add transaction'">
      <template #body>
        <TransactionForm :editing="editing" @saved="onSaved" />
      </template>
    </UModal>

    <UModal v-model:open="isImportOpen" title="Import transactions from CSV" class="max-w-full w-4/5 lg:w-[1000px]">
      <template #body>
        <ImportWizard @done="onImportDone" />
      </template>
    </UModal>
  </div>
</template>
