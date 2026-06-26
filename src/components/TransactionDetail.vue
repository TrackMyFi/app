<script setup lang="ts">
import { useAccountsStore } from '../stores/accounts'
import type { Transaction } from '../lib/types/Transaction'

defineProps<{ transaction: Transaction }>()

const accounts = useAccountsStore()

function accountName(id: number | null): string {
  if (id == null) return '—'
  return accounts.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

const money = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
</script>

<template>
  <dl class="grid grid-cols-3 gap-x-4 gap-y-3 text-sm">
    <dt class="text-gray-500">Date</dt>
    <dd class="col-span-2">{{ transaction.date }}</dd>

    <dt class="text-gray-500">Description</dt>
    <dd class="col-span-2">{{ transaction.description }}</dd>

    <dt class="text-gray-500">Amount</dt>
    <dd
      class="col-span-2 tabular-nums font-medium"
      :class="transaction.type === 'income' ? 'text-success' : transaction.type === 'expense' ? 'text-error' : ''"
    >
      {{ money(transaction.amount) }}
    </dd>

    <dt class="text-gray-500">Type</dt>
    <dd class="col-span-2 capitalize">{{ transaction.type }}</dd>

    <dt class="text-gray-500">Category</dt>
    <dd class="col-span-2">{{ transaction.category }}</dd>

    <dt class="text-gray-500">Account</dt>
    <dd class="col-span-2">
      {{ accountName(transaction.accountId) }}
      <span v-if="transaction.type === 'transfer'"> → {{ accountName(transaction.transferAccountId) }}</span>
    </dd>

    <template v-if="transaction.isContribution">
      <dt class="text-gray-500">{{ transaction.isWithdrawal ? 'Withdrawal' : 'Contribution' }}</dt>
      <dd class="col-span-2">Yes</dd>
    </template>

    <dt class="text-gray-500">Source</dt>
    <dd class="col-span-2">{{ transaction.importSource }}</dd>
  </dl>
</template>
