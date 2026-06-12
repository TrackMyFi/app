<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useAccountsStore } from '../stores/accounts'
import AccountForm from '../components/AccountForm.vue'
import BalanceForm from '../components/BalanceForm.vue'
import type { Account } from '../lib/types/Account'
import type { AccountBalance } from '../lib/types/AccountBalance'
import { confirm } from '@tauri-apps/plugin-dialog';

const store = useAccountsStore()

onMounted(async () => {
  await store.load()
})

const activeAccounts = computed(() =>
  store.accounts.filter((a: Account) => a.isActive),
)

const archivedAccounts = computed(() =>
  store.accounts.filter((a: Account) => !a.isActive),
)

function latestBalance(accountId: number): string {
  const balances = store.allBalances.filter(
    (b: AccountBalance) => b.accountId === accountId,
  )
  if (!balances.length) return '—'
  const latest = balances.reduce((best, b) =>
    b.recordedAt > best.recordedAt ? b : best,
  )
  return latest.balance.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

function accountBalances(accountId: number): AccountBalance[] {
  return store.allBalances
    .filter((b: AccountBalance) => b.accountId === accountId)
    .slice()
    .sort((a, b) => a.recordedAt.localeCompare(b.recordedAt))
}

async function archive(id: number) {
  await store.archive(id)
}

async function unarchive(id: number) {
  await store.unarchive(id)
}

async function remove(account: Account) {
  const ok = await confirm(
    `Permanently delete "${account.name}" and all of its balance snapshots? This cannot be undone.`,
    { title: 'Delete Account?', kind: 'warning' }
  );
  if (ok) await store.remove(account.id)
}
</script>

<template>
  <div class="p-6 max-w-4xl">
    <h1 class="text-2xl font-bold mb-6">Accounts</h1>

    <AccountForm />

    <div v-if="activeAccounts.length === 0" class="text-gray-500 text-sm mt-4">
      No active accounts. Add one above.
    </div>

    <div class="space-y-4">
      <UCard v-for="account in activeAccounts" :key="account.id">
        <template #header>
          <div class="flex items-center justify-between">
            <div>
              <span class="font-semibold text-base">{{ account.name }}</span>
              <span class="ml-2 text-sm text-gray-500">{{ account.type }}</span>
              <span v-if="account.institution" class="ml-2 text-sm text-gray-400">· {{ account.institution }}</span>
            </div>
            <div class="flex items-center gap-4">
              <span class="text-sm text-gray-500">
                FIRE: <span :class="account.includeInFireCalculations ? 'text-green-600 font-medium' : 'text-gray-400'">
                  {{ account.includeInFireCalculations ? 'Yes' : 'No' }}
                </span>
              </span>
              <span class="font-semibold">{{ latestBalance(account.id) }}</span>
              <UButton
                size="sm"
                color="error"
                variant="ghost"
                @click="archive(account.id)"
              >
                Archive
              </UButton>
            </div>
          </div>
        </template>

        <div class="space-y-4">
          <BalanceForm :account-id="account.id" />

          <div v-if="accountBalances(account.id).length > 0">
            <h3 class="text-sm font-medium text-gray-600 mb-2">Balance History</h3>
            <div class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead>
                  <tr class="text-left text-gray-500 border-b">
                    <th class="pb-1 pr-6 font-medium">Date</th>
                    <th class="pb-1 font-medium text-right">Balance</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="b in accountBalances(account.id)"
                    :key="b.id"
                    class="border-b border-gray-100 last:border-0"
                  >
                    <td class="py-1 pr-6 text-gray-600">{{ b.recordedAt }}</td>
                    <td class="py-1 text-right font-mono">
                      {{ b.balance.toLocaleString('en-US', { style: 'currency', currency: 'USD' }) }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </UCard>
    </div>

    <div v-if="archivedAccounts.length > 0" class="mt-10">
      <h2 class="text-lg font-semibold mb-1">Archived</h2>
      <p class="text-sm text-gray-500 mb-4">
        Archived accounts are excluded from your dashboard totals. Restore one to include it again,
        or permanently delete it (removes the account and its balance history).
      </p>
      <div class="space-y-2">
        <UCard v-for="account in archivedAccounts" :key="account.id">
          <div class="flex items-center justify-between">
            <div>
              <span class="font-medium">{{ account.name }}</span>
              <span class="ml-2 text-sm text-gray-500">{{ account.type }}</span>
              <span v-if="account.institution" class="ml-2 text-sm text-gray-400">· {{ account.institution }}</span>
              <span class="ml-2 text-sm text-gray-400">· {{ latestBalance(account.id) }}</span>
            </div>
            <div class="flex items-center gap-2">
              <UButton size="sm" variant="ghost" @click="unarchive(account.id)">Restore</UButton>
              <UButton size="sm" color="error" variant="soft" @click="remove(account)">Delete</UButton>
            </div>
          </div>
        </UCard>
      </div>
    </div>
  </div>
</template>
