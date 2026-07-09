<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useHsaExpensesStore } from '../stores/hsaExpenses'
import { useAccountsStore } from '../stores/accounts'
import { hsaCategoryItems, labelForHsaCategory, colorForHsaCategory } from '../lib/hsa/constants'
import { totalExpenses, unreimbursedTotal, reimbursedTotal } from '../lib/hsa/rollups'
import HsaExpenseForm from '../components/HsaExpenseForm.vue'
import HsaExpenseDetail from '../components/HsaExpenseDetail.vue'
import DateInput from '../components/DateInput.vue'
import type { HsaExpense } from '../lib/types/HsaExpense'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'

const store = useHsaExpensesStore()
const accountsStore = useAccountsStore()
const toast = useToast()
const { error, run, retry } = usePageData()

const isModalOpen = ref(false)
const editing = ref<HsaExpense | null>(null)

function openAdd() { editing.value = null; isModalOpen.value = true }
function openEdit(e: HsaExpense) { editing.value = e; isModalOpen.value = true }
function onSaved() { isModalOpen.value = false }

watch(isModalOpen, (open) => { if (!open) editing.value = null })

// ---- detail view ----
const isDetailOpen = ref(false)
const viewingExpense = ref<HsaExpense | null>(null)

function openDetail(e: HsaExpense) {
  viewingExpense.value = e
  isDetailOpen.value = true
}

watch(isDetailOpen, (open) => { if (!open) viewingExpense.value = null })

function onDetailEdit(e: HsaExpense) {
  isDetailOpen.value = false
  openEdit(e)
}

const removingId = ref<number | null>(null)
async function removeRow(e: HsaExpense) {
  const ok = await confirm(`Delete "${e.description}" on ${e.date}?`, { title: 'Delete HSA expense' })
  if (!ok) return
  removingId.value = e.id
  try {
    await store.remove(e.id)
  } catch (err) {
    toast.add({ title: 'Failed to delete expense', description: String(err), color: 'error' })
  } finally {
    removingId.value = null
  }
}

// ---- filters ----
// 'all' is a sentinel (Reka UI's <SelectItem> rejects an empty-string value).
const categoryFilter = ref('all')
const statusFilter = ref('all')
const startDate = ref('')
const endDate = ref('')
const search = ref('')

async function applyFilters() {
  await store.setFilter({
    category: categoryFilter.value === 'all' ? null : categoryFilter.value,
    reimbursed: statusFilter.value === 'all' ? null : statusFilter.value === 'reimbursed',
    startDate: startDate.value || null,
    endDate: endDate.value || null,
    search: search.value || null,
  })
}
async function clearFilters() {
  categoryFilter.value = 'all'
  statusFilter.value = 'all'
  startDate.value = ''
  endDate.value = ''
  search.value = ''
  await store.setFilter({ category: null, reimbursed: null, startDate: null, endDate: null, search: null })
}

const categoryItems = [{ label: 'All categories', value: 'all' }, ...hsaCategoryItems]
const statusItems = [
  { label: 'All expenses', value: 'all' },
  { label: 'Unreimbursed', value: 'unreimbursed' },
  { label: 'Reimbursed', value: 'reimbursed' },
]

// ---- rollups ----
const expenses = computed(() => store.hsaExpenses)

const totals = computed(() => ({
  unreimbursed: unreimbursedTotal(expenses.value),
  reimbursed: reimbursedTotal(expenses.value),
  total: totalExpenses(expenses.value),
  count: expenses.value.length,
}))

function accountName(accountId: number | null): string | null {
  if (accountId == null) return null
  return accountsStore.accounts.find((a) => a.id === accountId)?.name ?? `Account #${accountId}`
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}
function fmtDate(iso: string): string {
  return DateTime.fromISO(iso).toLocaleString(DateTime.DATE_MED)
}

onMounted(() => run(async () => {
  await accountsStore.loadList()
  await store.load()
  categoryFilter.value = store.filter.category ?? 'all'
  statusFilter.value = store.filter.reimbursed == null ? 'all' : store.filter.reimbursed ? 'reimbursed' : 'unreimbursed'
  startDate.value = store.filter.startDate ?? ''
  endDate.value = store.filter.endDate ?? ''
  search.value = store.filter.search ?? ''
}))
</script>

<template>
  <div class="p-6 space-y-4">
    <PageError v-if="error" :message="error" @retry="retry" />

    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">HSA Receipts</h1>
        <p class="text-sm text-muted">Eligible medical expenses &amp; receipts you can reimburse tax-free, anytime.</p>
      </div>
      <UButton icon="i-ph-plus" @click="openAdd">Add expense</UButton>
    </div>

    <!-- Rollup stats -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-3">
      <div class="border border-default rounded-lg p-4">
        <p class="text-xs uppercase tracking-wide text-muted">Receipt bank (unreimbursed)</p>
        <p class="text-xl font-semibold tabular-nums mt-1 text-success">{{ money(totals.unreimbursed) }}</p>
      </div>
      <div class="border border-default rounded-lg p-4">
        <p class="text-xs uppercase tracking-wide text-muted">Reimbursed</p>
        <p class="text-xl font-semibold tabular-nums mt-1">{{ money(totals.reimbursed) }}</p>
      </div>
      <div class="border border-default rounded-lg p-4">
        <p class="text-xs uppercase tracking-wide text-muted">Total expenses</p>
        <p class="text-xl font-semibold tabular-nums mt-1">{{ money(totals.total) }}</p>
      </div>
      <div class="border border-default rounded-lg p-4">
        <p class="text-xs uppercase tracking-wide text-muted">Receipts logged</p>
        <p class="text-xl font-semibold tabular-nums mt-1">{{ totals.count }}</p>
      </div>
    </div>

    <!-- Filters -->
    <div class="flex flex-wrap gap-2 items-end">
      <div>
        <p class="text-xs text-muted mb-1">Category</p>
        <USelect v-model="categoryFilter" :items="categoryItems" class="w-40" />
      </div>
      <div>
        <p class="text-xs text-muted mb-1">Status</p>
        <USelect v-model="statusFilter" :items="statusItems" class="w-40" />
      </div>
      <div>
        <p class="text-xs text-muted mb-1">From</p>
        <DateInput v-model="startDate" />
      </div>
      <div>
        <p class="text-xs text-muted mb-1">To</p>
        <DateInput v-model="endDate" />
      </div>
      <UInput v-model="search" placeholder="Search" class="w-44" />
      <UButton @click="applyFilters">Apply</UButton>
      <UButton variant="ghost" @click="clearFilters">Clear</UButton>
    </div>

    <p v-if="expenses.length === 0" class="text-sm text-muted py-8 text-center">
      No HSA expenses yet. Log a receipt above to start building your tax-free receipt bank.
    </p>

    <div v-else class="border border-default rounded-lg overflow-hidden">
      <table class="w-full text-sm">
        <tbody>
          <tr
            v-for="e in expenses"
            :key="e.id"
            class="not-first:border-t border-default cursor-pointer hover:bg-muted/30 transition-colors"
            @click="openDetail(e)"
          >
            <td class="px-4 py-2 whitespace-nowrap text-muted">{{ fmtDate(e.date) }}</td>
            <td class="px-4 py-2">
              <UBadge :color="colorForHsaCategory(e.category)" variant="subtle" size="xs">
                {{ labelForHsaCategory(e.category) }}
              </UBadge>
            </td>
            <td class="px-4 py-2">
              <span>{{ e.description }}</span>
              <span v-if="e.person" class="text-muted"> · {{ e.person }}</span>
              <span v-if="e.provider" class="text-muted"> · {{ e.provider }}</span>
            </td>
            <td class="px-4 py-2 whitespace-nowrap">
              <UBadge v-if="e.reimbursed" color="neutral" variant="subtle" size="xs" icon="i-ph-check">
                Reimbursed
              </UBadge>
              <UBadge v-else color="success" variant="subtle" size="xs" icon="i-ph-piggy-bank">
                Receipt bank
              </UBadge>
            </td>
            <td class="px-4 py-2 text-right tabular-nums">{{ money(e.amount) }}</td>
            <td class="px-4 py-2 text-right whitespace-nowrap" @click.stop>
              <UTooltip v-if="e.hasAttachment" text="Has attachment">
                <span class="i-ph-paperclip inline-block text-muted mr-1 align-middle" />
              </UTooltip>
              <UButton size="xs" variant="ghost" icon="i-ph-pencil" @click="openEdit(e)" />
              <UButton
                size="xs"
                variant="ghost"
                color="error"
                icon="i-ph-trash"
                :loading="removingId === e.id"
                :disabled="removingId !== null"
                @click="removeRow(e)"
              />
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit HSA expense' : 'Add HSA expense'" class="sm:w-[560px] max-w-full">
      <template #body>
        <HsaExpenseForm :editing="editing" @saved="onSaved" />
      </template>
    </UModal>

    <UModal
      v-if="viewingExpense"
      v-model:open="isDetailOpen"
      :title="viewingExpense.description"
      class="sm:w-[600px] max-w-full"
    >
      <template #body>
        <HsaExpenseDetail
          :expense="viewingExpense"
          :account-name="accountName(viewingExpense.accountId)"
          @edit="onDetailEdit"
        />
      </template>
    </UModal>
  </div>
</template>
