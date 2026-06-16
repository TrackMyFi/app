<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useTransactionsStore } from '../stores/transactions'
import { useAccountsStore } from '../stores/accounts'
import { transactionTypeItems, categoryItems, labelForTransactionType, labelForCategory } from '../lib/transactions/constants'
import TransactionForm from '../components/TransactionForm.vue'
import ImportWizard from '../components/ImportWizard.vue'
import type { Transaction } from '../lib/types/Transaction'
import { confirm } from '@tauri-apps/plugin-dialog'

const store = useTransactionsStore()
const accountsStore = useAccountsStore()

const isModalOpen = ref(false)
const editing = ref<Transaction | null>(null)

function openAdd() { editing.value = null; isModalOpen.value = true }
function openEdit(t: Transaction) { editing.value = t; isModalOpen.value = true }
function onSaved() { isModalOpen.value = false }

const isImportOpen = ref(false)
function onImportDone() { isImportOpen.value = false }

async function removeRow(t: Transaction) {
  const ok = await confirm(`Delete "${t.description}"?`, { title: 'Delete transaction' })
  if (ok) await store.remove(t.id)
}

const accountId = ref<number | undefined>(undefined)
const type = ref<string | undefined>(undefined)
const category = ref<string | undefined>(undefined)
const search = ref('')

function accountName(id: number | null): string {
  if (id == null) return '—'
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

async function applyFilters() {
  await store.setFilter({
    accountId: accountId.value ?? null,
    type: type.value ?? null,
    category: category.value ?? null,
    search: search.value || null,
  })
}

async function clearFilters() {
  accountId.value = undefined
  type.value = undefined
  category.value = undefined
  search.value = ''
  await store.setFilter({ accountId: null, type: null, category: null, search: null })
}

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

onMounted(async () => {
  await accountsStore.load()
  await store.load()
  accountId.value = store.filter.accountId ?? undefined
  type.value = store.filter.type ?? undefined
  category.value = store.filter.category ?? undefined
  search.value = store.filter.search ?? ''
})
</script>

<template>
  <div class="p-6 space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Transactions</h1>
      <div class="flex gap-2">
        <UButton variant="soft" icon="i-ph-upload" @click="isImportOpen = true">Import CSV</UButton>
        <UButton icon="i-ph-plus" @click="openAdd">Add transaction</UButton>
      </div>
    </div>

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

    <div class="flex gap-6 text-sm">
      <span>Income: <strong class="text-green-600">{{ money(store.page.totalIncome) }}</strong></span>
      <span>Expense: <strong class="text-red-600">{{ money(store.page.totalExpense) }}</strong></span>
      <span>Net: <strong>{{ money(store.page.net) }}</strong></span>
      <span class="text-muted">{{ store.page.totalCount }} rows</span>
    </div>

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
