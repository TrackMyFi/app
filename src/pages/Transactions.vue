<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useTransactionsStore } from '../stores/transactions'
import { useAccountsStore } from '../stores/accounts'
import { TRANSACTION_TYPES, CATEGORIES } from '../lib/transactions/constants'

const store = useTransactionsStore()
const accountsStore = useAccountsStore()

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

const rows = computed(() => store.page.rows)

onMounted(async () => {
  await accountsStore.load()
  await store.load()
})
</script>

<template>
  <div class="p-6 space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Transactions</h1>
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
        :items="TRANSACTION_TYPES.map((t) => ({ label: t, value: t }))"
        placeholder="All types"
        class="w-36"
      />
      <USelect
        v-model="category"
        :items="CATEGORIES.map((c) => ({ label: c, value: c }))"
        placeholder="All categories"
        class="w-40"
      />
      <UInput v-model="search" placeholder="Search description" class="w-52" />
      <UButton @click="applyFilters">Apply</UButton>
    </div>

    <div class="flex gap-6 text-sm">
      <span>Income: <strong class="text-green-600">{{ money(store.page.totalIncome) }}</strong></span>
      <span>Expense: <strong class="text-red-600">{{ money(store.page.totalExpense) }}</strong></span>
      <span>Net: <strong>{{ money(store.page.net) }}</strong></span>
      <span class="text-muted">{{ store.page.totalCount }} rows</span>
    </div>

    <table class="w-full text-sm">
      <thead class="text-left text-muted border-b border-default">
        <tr>
          <th class="py-2">Date</th>
          <th>Description</th>
          <th>Account</th>
          <th>Type</th>
          <th>Category</th>
          <th class="text-right">Amount</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="t in rows" :key="t.id" class="border-b border-default/50">
          <td class="py-2">{{ t.date }}</td>
          <td>{{ t.description }}</td>
          <td>
            {{ accountName(t.accountId) }}
            <span v-if="t.type === 'transfer'"> → {{ accountName(t.transferAccountId) }}</span>
          </td>
          <td>{{ t.type }}</td>
          <td>{{ t.category }}</td>
          <td class="text-right tabular-nums">{{ money(t.amount) }}</td>
        </tr>
        <tr v-if="!rows.length">
          <td colspan="6" class="py-6 text-center text-muted">No transactions yet.</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
