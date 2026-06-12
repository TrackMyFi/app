<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useAccountsStore } from '../stores/accounts'
import AccountForm from '../components/AccountForm.vue'
import BalanceForm from '../components/BalanceForm.vue'
import BalanceRow from '../components/BalanceRow.vue'
import type { Account } from '../lib/types/Account'
import type { AccountBalance } from '../lib/types/AccountBalance'
import { confirm } from '@tauri-apps/plugin-dialog';

const store = useAccountsStore()

const isAccountModalOpen = ref(false)
const editingAccount = ref<Account | null>(null)

function openAdd() {
  editingAccount.value = null
  isAccountModalOpen.value = true
}

function openEdit(account: Account) {
  editingAccount.value = account
  isAccountModalOpen.value = true
}

function onAccountSaved() {
  isAccountModalOpen.value = false
}

watch(isAccountModalOpen, (open) => {
  if (!open) editingAccount.value = null
})

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

    <div class="mb-6">
      <UButton icon="i-lucide-plus" @click="openAdd">Add Account</UButton>
    </div>

    <UModal
      v-model:open="isAccountModalOpen"
      :title="editingAccount ? 'Edit Account' : 'Add Account'"
    >
      <template #body>
        <AccountForm
          :key="editingAccount?.id ?? 'new'"
          :account="editingAccount ?? undefined"
          @saved="onAccountSaved"
        />
      </template>
    </UModal>

    <div v-if="activeAccounts.length === 0" class="text-gray-500 text-sm mt-4">
      No active accounts. Click "Add Account" to get started.
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
                variant="ghost"
                @click="openEdit(account)"
              >
                Edit
              </UButton>
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
                    <th class="pb-1 font-medium text-right"></th>
                  </tr>
                </thead>
                <tbody>
                  <BalanceRow
                    v-for="b in accountBalances(account.id)"
                    :key="b.id"
                    :balance="b"
                  />
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
