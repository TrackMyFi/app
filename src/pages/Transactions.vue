<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { useToast } from '@nuxt/ui/composables'
import { useTransactionsStore } from '../stores/transactions'
import { useAccountsStore } from '../stores/accounts'
import { transactionTypeItems, categoryItems, labelForCategory } from '../lib/transactions/constants'
import { classifyFlow, cashFlowTotals, savingsRate, type FlowDirection } from '../lib/transactions/flow'
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
const toast = useToast()

// ─── Modals ───────────────────────────────────────────────────────────────────

const isModalOpen = ref(false)
const editing = ref<Transaction | null>(null)

function openAdd() { editing.value = null; isModalOpen.value = true }
function openEdit(t: Transaction) { editing.value = t; isModalOpen.value = true }
async function onSaved() { isModalOpen.value = false; await loadYearData() }

const isImportOpen = ref(false)
async function onImportDone() { isImportOpen.value = false; await loadYearData() }

const removingId = ref<number | null>(null)

async function removeRow(t: Transaction) {
  const ok = await confirm(`Delete "${t.description}"?`, { title: 'Delete transaction' })
  if (!ok) return
  removingId.value = t.id
  try {
    await store.remove(t.id)
    await loadYearData()
  } catch (err) {
    toast.add({ title: 'Failed to delete transaction', description: String(err), color: 'error' })
  } finally {
    removingId.value = null
  }
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

const monthlyTotals = computed(() => cashFlowTotals(store.page.rows, accountsStore.accounts))
const annualTotals = computed(() => cashFlowTotals(yearTransactions.value, accountsStore.accounts))

function formatSavingsRate(totals: { income: number; expense: number; savings: number; net: number }): string {
  const rate = savingsRate(totals)
  return rate == null ? '—' : (rate * 100).toFixed(1) + '%'
}

const monthlySavingsRate = computed(() => formatSavingsRate(monthlyTotals.value))
const annualSavingsRate = computed(() => formatSavingsRate(annualTotals.value))

// ─── Table ─────────────────────────────────────────────────────────────────────

const rows = computed(() => store.page.rows)

const columns = [
  { accessorKey: 'date', header: 'Date' },
  { accessorKey: 'description', header: 'Description' },
  { id: 'account', header: 'Account' },
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

// ─── Amount direction indicator ──────────────────────────────────────────────

const DIRECTION_COLOR: Record<FlowDirection, string> = {
  inflow: 'text-success',
  outflow: 'text-error',
  neutral: 'text-muted',
}

// Transfers always use the ⇄ glyph; colour encodes the asset/liability direction.
// Plain income/expense use a horizontal arrow: right = money in, left = money out.
function flowIcon(t: Transaction): string {
  const { direction, isTransfer } = classifyFlow(t, accountsStore.accounts)
  if (isTransfer) return 'i-ph-arrows-left-right'
  return direction === 'inflow' ? 'i-ph-arrow-right' : 'i-ph-arrow-left'
}

function flowColor(t: Transaction): string {
  return DIRECTION_COLOR[classifyFlow(t, accountsStore.accounts).direction]
}

function directionLabel(t: Transaction): string {
  const { direction, isTransfer } = classifyFlow(t, accountsStore.accounts)
  if (!isTransfer) return direction === 'inflow' ? 'Income' : 'Expense'
  if (direction === 'inflow') return 'Transfer in'
  if (direction === 'outflow') return 'Transfer out'
  return 'Transfer'
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
        <div class="flex gap-1">
          <UButton
            size="xs"
            :variant="chartMode === 'monthly' ? 'subtle' : 'ghost'"
            :color="chartMode === 'monthly' ? 'primary' : 'neutral'"
            @click="chartMode = 'monthly'"
          >Monthly</UButton>
          <UButton
            size="xs"
            :variant="chartMode === 'annual' ? 'subtle' : 'ghost'"
            :color="chartMode === 'annual' ? 'primary' : 'neutral'"
            @click="chartMode = 'annual'"
          >Annual</UButton>
        </div>
      </div>
      <p class="text-sm text-muted mb-3">{{ chartTitle }}</p>
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
    <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
      <div class="border border-default rounded-lg p-4">
        <div class="flex items-center justify-between mb-3">
          <p class="text-sm font-medium text-heading">{{ monthLabel }}</p>
          <span class="text-xs text-muted">{{ store.page.totalCount }} transactions</span>
        </div>
        <div class="grid grid-cols-5 gap-3">
          <div>
            <p class="text-base font-semibold tabular-nums text-success">{{ money(monthlyTotals.income) }}</p>
            <p class="text-xs text-muted mt-0.5">Income</p>
          </div>
          <div>
            <p class="text-base font-semibold tabular-nums text-error">{{ money(monthlyTotals.expense) }}</p>
            <p class="text-xs text-muted mt-0.5">Expense</p>
          </div>
          <div>
            <p class="text-base font-semibold tabular-nums text-info">{{ money(monthlyTotals.savings) }}</p>
            <p class="text-xs text-muted mt-0.5">Savings</p>
          </div>
          <div>
            <p class="text-base font-semibold tabular-nums">{{ money(monthlyTotals.net) }}</p>
            <p class="text-xs text-muted mt-0.5">Net</p>
          </div>
          <div class="border-l border-default pl-3">
            <p class="text-xl font-bold tabular-nums" :class="monthlyTotals.savings > 0 ? 'text-info' : 'text-muted'">{{ monthlySavingsRate }}</p>
            <p class="text-xs text-muted mt-0.5">Savings Rate</p>
          </div>
        </div>
      </div>
      <div class="border border-default rounded-lg p-4">
        <div class="flex items-center justify-between mb-3">
          <p class="text-sm font-medium text-heading">{{ yearLabel }} Annual</p>
        </div>
        <div class="grid grid-cols-5 gap-3">
          <div>
            <p class="text-base font-semibold tabular-nums text-success">{{ money(annualTotals.income) }}</p>
            <p class="text-xs text-muted mt-0.5">Income</p>
          </div>
          <div>
            <p class="text-base font-semibold tabular-nums text-error">{{ money(annualTotals.expense) }}</p>
            <p class="text-xs text-muted mt-0.5">Expense</p>
          </div>
          <div>
            <p class="text-base font-semibold tabular-nums text-info">{{ money(annualTotals.savings) }}</p>
            <p class="text-xs text-muted mt-0.5">Savings</p>
          </div>
          <div>
            <p class="text-base font-semibold tabular-nums">{{ money(annualTotals.net) }}</p>
            <p class="text-xs text-muted mt-0.5">Net</p>
          </div>
          <div class="border-l border-default pl-3">
            <p class="text-xl font-bold tabular-nums" :class="annualTotals.savings > 0 ? 'text-info' : 'text-muted'">{{ annualSavingsRate }}</p>
            <p class="text-xs text-muted mt-0.5">Savings Rate</p>
          </div>
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
        <template #description-cell="{ row }">
          <span class="block max-w-[300px] truncate" :title="row.original.description">{{ row.original.description }}</span>
        </template>
        <template #account-cell="{ row }">
          {{ accountName(row.original.accountId) }}
          <span v-if="row.original.type === 'transfer'"> → {{ accountName(row.original.transferAccountId) }}</span>
        </template>
        <template #category-cell="{ row }">
          <div class="flex items-center gap-1.5">
            <UBadge
              v-if="row.original.isContribution"
              color="info"
              variant="subtle"
              size="sm"
              icon="i-ph-piggy-bank"
            >Contribution</UBadge>
            <span v-else>{{ labelForCategory(row.original.category) }}</span>
          </div>
        </template>
        <template #amount-cell="{ row }">
          <span class="inline-flex items-center justify-end gap-1.5" :class="flowColor(row.original)">
            <UIcon :name="flowIcon(row.original)" class="size-4 shrink-0" :title="directionLabel(row.original)" />
            {{ money(row.original.amount) }}
          </span>
        </template>
        <template #actions-cell="{ row }">
          <UButton size="xs" variant="ghost" icon="i-ph-pencil" @click="openEdit(row.original)" />
          <UButton size="xs" variant="ghost" color="error" icon="i-ph-trash" :loading="removingId === row.original.id" :disabled="removingId !== null" @click="removeRow(row.original)" />
        </template>
      </UTable>
    </div>

    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit transaction' : 'Add transaction'">
      <template #body>
        <TransactionForm :editing="editing" @saved="onSaved" @cancel="isModalOpen = false" />
      </template>
    </UModal>

    <UModal v-model:open="isImportOpen" title="Import transactions from CSV" class="max-w-full w-4/5 lg:w-[1000px]">
      <template #body>
        <ImportWizard @done="onImportDone" />
      </template>
    </UModal>
  </div>
</template>
