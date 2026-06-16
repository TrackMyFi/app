<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { useAccountsStore } from '../stores/accounts'
import { labelForAccountType } from '../lib/accountTypes'
import AccountForm from '../components/AccountForm.vue'
import BalanceForm from '../components/BalanceForm.vue'
import DateInput from '../components/DateInput.vue'
import CurrencyInput from '../components/CurrencyInput.vue'
import type { Account } from '../lib/types/Account'
import type { AccountBalance } from '../lib/types/AccountBalance'
import { latestSnapshot, byRecencyDesc } from '../lib/balances/recency'
import TransactionDetail from '../components/TransactionDetail.vue'
import type { Transaction } from '../lib/types/Transaction'
import { getTransaction } from '../lib/api/transactions'
import { confirm } from '@tauri-apps/plugin-dialog';

const store = useAccountsStore()
const toast = useToast()

const editingBalanceId = ref<number | null>(null)
const draftBalance = ref<number>(0)
const draftDate = ref<string>('')

const balanceColumns = [
  { accessorKey: 'recordedAt', header: 'Date' },
  { id: 'balance', header: 'Balance', meta: { class: { th: 'text-right' } } },
  { id: 'actions', header: '' },
]

const formatted = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })

function startEditBalance(b: AccountBalance) {
  draftBalance.value = b.balance
  draftDate.value = b.recordedAt
  editingBalanceId.value = b.id
}

function cancelEditBalance() {
  editingBalanceId.value = null
}

async function saveBalance(b: AccountBalance) {
  await store.updateBalanceSnapshot({ id: b.id, balance: draftBalance.value, recordedAt: draftDate.value })
  toast.add({ title: 'Balance updated', color: 'success' })
  editingBalanceId.value = null
}

async function removeBalance(b: AccountBalance) {
  const ok = await confirm(
    `Delete this balance snapshot from ${b.recordedAt}? This cannot be undone.`,
    { title: 'Delete Snapshot?', kind: 'warning' },
  )
  if (ok) await store.removeBalanceSnapshot(b.id)
}

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

const isTransactionModalOpen = ref(false)
const viewingTransaction = ref<Transaction | null>(null)

async function openTransaction(id: number) {
  viewingTransaction.value = await getTransaction(id)
  isTransactionModalOpen.value = true
}

watch(isTransactionModalOpen, (open) => {
  if (!open) viewingTransaction.value = null
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
  const latest = latestSnapshot(balances)
  if (!latest) return '—'
  return latest.balance.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

function accountBalances(accountId: number): AccountBalance[] {
  return store.allBalances
    .filter((b: AccountBalance) => b.accountId === accountId)
    .slice()
    .sort(byRecencyDesc)
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
      <UButton icon="i-ph-plus" @click="openAdd">Add Account</UButton>
    </div>

    <UModal
      v-model:open="isAccountModalOpen"
      :title="editingAccount ? 'Edit Account' : 'Add Account'"
      class="w-112"
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
              <span class="ml-2 text-sm text-gray-500">{{ labelForAccountType(account.type) }}</span>
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
            <UTable :data="accountBalances(account.id)" :columns="balanceColumns">
              <template #recordedAt-cell="{ row }">
                <DateInput v-if="editingBalanceId === row.original.id" v-model="draftDate" />
                <span v-else class="text-gray-600">{{ row.original.recordedAt }}</span>
              </template>
              <template #balance-cell="{ row }">
                <div class="flex justify-end">
                  <CurrencyInput v-if="editingBalanceId === row.original.id" v-model="draftBalance" class="w-32" />
                  <span v-else class="font-mono">{{ formatted(row.original.balance) }}</span>
                </div>
              </template>
              <template #actions-cell="{ row }">
                <div class="flex justify-end gap-1">
                  <template v-if="editingBalanceId === row.original.id">
                    <UButton size="xs" variant="ghost" @click="saveBalance(row.original)">Save</UButton>
                    <UButton size="xs" variant="ghost" color="neutral" @click="cancelEditBalance">Cancel</UButton>
                  </template>
                  <template v-else>
                    <UButton
                      v-if="row.original.linkedTransactionId != null"
                      size="xs"
                      variant="ghost"
                      icon="i-ph-receipt"
                      title="View linked transaction"
                      @click="openTransaction(row.original.linkedTransactionId)"
                    />
                    <UButton size="xs" variant="ghost" @click="startEditBalance(row.original)">Edit</UButton>
                    <UButton size="xs" variant="ghost" color="error" @click="removeBalance(row.original)">Delete</UButton>
                  </template>
                </div>
              </template>
            </UTable>
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
              <span class="ml-2 text-sm text-gray-500">{{ labelForAccountType(account.type) }}</span>
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

    <UModal v-model:open="isTransactionModalOpen" title="Transaction details">
      <template #body>
        <TransactionDetail v-if="viewingTransaction" :transaction="viewingTransaction" />
      </template>
    </UModal>
  </div>
</template>
