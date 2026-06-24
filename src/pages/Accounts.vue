<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { DateTime } from 'luxon'
import { useToast } from '@nuxt/ui/composables'
import { useAccountsStore } from '../stores/accounts'
import { labelForAccountType, isLiability, isEquity } from '../lib/accountTypes'
import AccountForm from '../components/AccountForm.vue'
import StatCard from '../components/StatCard.vue'
import type { Account } from '../lib/types/Account'
import { confirm } from '@tauri-apps/plugin-dialog'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'

const store = useAccountsStore()
const router = useRouter()
const toast = useToast()
const { error, run, retry } = usePageData()

onMounted(() => run(() => store.loadList()))

const fmt = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

const latestBalanceMap = computed(() =>
  new Map(store.latestBalances.map(b => [b.accountId, b.balance]))
)

const latestDateMap = computed(() =>
  new Map(store.latestBalances.map(b => [b.accountId, b.recordedAt]))
)

const activeAccounts = computed(() => store.accounts.filter(a => a.isActive))
const archivedAccounts = computed(() => store.accounts.filter(a => !a.isActive))
const equityAccounts = computed(() => activeAccounts.value.filter(a => isEquity(a.type)))
const fireAccounts = computed(() => activeAccounts.value.filter(a => a.includeInFireCalculations && !isEquity(a.type)))
const nonFireAccounts = computed(() => activeAccounts.value.filter(a => !a.includeInFireCalculations && !isEquity(a.type)))

const signedBalance = (a: Account) => {
  const b = latestBalanceMap.value.get(a.id) ?? 0
  return isLiability(a.type) ? -b : b
}

const netWorth = computed(() =>
  activeAccounts.value.reduce((s, a) => s + signedBalance(a), 0)
)
const fireTotal = computed(() =>
  fireAccounts.value.reduce((s, a) => s + signedBalance(a), 0)
)
const nonFireTotal = computed(() =>
  nonFireAccounts.value.reduce((s, a) => s + signedBalance(a), 0)
)
const equityTotal = computed(() =>
  equityAccounts.value.reduce((s, a) => s + signedBalance(a), 0)
)

function latestBalance(accountId: number) {
  const b = latestBalanceMap.value.get(accountId)
  return b != null ? b.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 }) : '—'
}

function formatBalanceDate(accountId: number) {
  const d = latestDateMap.value.get(accountId)
  if (!d) return null
  const dt = DateTime.fromISO(d)
  const now = DateTime.now()
  return dt.year === now.year ? dt.toFormat('MMM d') : dt.toFormat('MMM d, yyyy')
}

function isStaleBalance(accountId: number) {
  const d = latestDateMap.value.get(accountId)
  if (!d) return false
  return DateTime.now().diff(DateTime.fromISO(d), 'days').days > 30
}

function navigate(account: Account) {
  router.push({ name: 'account-detail', params: { id: account.id } })
}

// Add / Edit account modal
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

watch(isAccountModalOpen, open => { if (!open) editingAccount.value = null })

function onAccountSaved() {
  isAccountModalOpen.value = false
}

// Archive / Unarchive / Delete
const busyAccountId = ref<number | null>(null)

async function archive(id: number) {
  busyAccountId.value = id
  try {
    await store.archive(id)
  } catch (err) {
    toast.add({ title: 'Failed to archive account', description: String(err), color: 'error' })
  } finally {
    busyAccountId.value = null
  }
}

async function unarchive(id: number) {
  busyAccountId.value = id
  try {
    await store.unarchive(id)
  } catch (err) {
    toast.add({ title: 'Failed to restore account', description: String(err), color: 'error' })
  } finally {
    busyAccountId.value = null
  }
}

async function remove(account: Account) {
  const ok = await confirm(
    `Permanently delete "${account.name}" and all of its balance snapshots? This cannot be undone.`,
    { title: 'Delete Account?', kind: 'warning' },
  )
  if (!ok) return
  busyAccountId.value = account.id
  try {
    await store.remove(account.id)
  } catch (err) {
    toast.add({ title: 'Failed to delete account', description: String(err), color: 'error' })
  } finally {
    busyAccountId.value = null
  }
}

// Archived section toggle
const showArchived = ref(false)

// Dropdown menu items per account
function activeMenuItems(account: Account) {
  return [[
    { label: 'Edit', icon: 'i-ph-pencil', onSelect: () => openEdit(account) },
    { label: 'Archive', icon: 'i-ph-archive', onSelect: () => archive(account.id) },
  ]]
}

function archivedMenuItems(account: Account) {
  return [[
    { label: 'Restore', icon: 'i-ph-arrow-counter-clockwise', onSelect: () => unarchive(account.id) },
    { label: 'Delete', icon: 'i-ph-trash', color: 'error' as const, onSelect: () => remove(account) },
  ]]
}
</script>

<template>
  <div class="p-6">
    <PageError v-if="error" :message="error" @retry="retry" class="mb-4" />
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-2xl font-bold">Accounts</h1>
      <UButton icon="i-ph-plus" @click="openAdd">Add Account</UButton>
    </div>

    <!-- Stats -->
    <div :class="['grid gap-4 mb-8', equityAccounts.length > 0 ? 'grid-cols-4' : 'grid-cols-3']">
      <StatCard label="Net Worth" :value="fmt(netWorth)" />
      <StatCard label="FIRE Accounts" :value="fmt(fireTotal)" />
      <StatCard label="Non-FIRE Accounts" :value="fmt(nonFireTotal)" />
      <StatCard v-if="equityAccounts.length > 0" label="Equity" :value="fmt(equityTotal)" />
    </div>

    <!-- Empty state: no accounts at all -->
    <div v-if="store.accounts.length === 0" class="border border-dashed border-default rounded-lg p-8 text-center mb-8">
      <UIcon name="i-ph-wallet" class="w-8 h-8 text-muted mx-auto mb-3" />
      <p class="text-sm font-medium mb-1">No accounts yet</p>
      <p class="text-sm text-muted">Add your first account above to start tracking your FIRE progress.</p>
    </div>

    <!-- Empty state: all accounts archived -->
    <div v-else-if="activeAccounts.length === 0" class="border border-dashed border-default rounded-lg p-8 text-center mb-8">
      <UIcon name="i-ph-archive" class="w-8 h-8 text-muted mx-auto mb-3" />
      <p class="text-sm font-medium mb-1">All accounts are archived</p>
      <p class="text-sm text-muted">Restore an account from the list below to see it here.</p>
    </div>

    <!-- FIRE Accounts -->
    <div v-if="fireAccounts.length > 0" class="mb-8">
      <p class="text-xs text-muted font-medium mb-2">FIRE Accounts</p>
      <div class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_52px] bg-elevated px-4 py-2 border-b border-default">
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
          <span />
        </div>
        <div
          v-for="account in fireAccounts"
          :key="account.id"
          class="grid grid-cols-[1fr_160px_140px_52px] items-center px-4 py-3 border-b border-default last:border-b-0 cursor-pointer hover:bg-elevated/50 transition-colors"
          @click="navigate(account)"
        >
          <div>
            <p class="text-sm font-medium">{{ account.name }}</p>
            <p v-if="account.institution" class="text-xs text-muted">{{ account.institution }}</p>
          </div>
          <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>
          <div class="text-right">
            <p class="text-sm font-semibold font-mono">{{ latestBalance(account.id) }}</p>
            <p v-if="formatBalanceDate(account.id)" :class="['text-xs', isStaleBalance(account.id) ? 'text-warning' : 'text-muted']">{{ formatBalanceDate(account.id) }}</p>
          </div>
          <div class="flex justify-end" @click.stop>
            <UDropdownMenu :items="activeMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" aria-label="Account options" :loading="busyAccountId === account.id" :disabled="busyAccountId !== null" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Non-FIRE Accounts -->
    <div v-if="nonFireAccounts.length > 0" class="mb-8">
      <p class="text-xs text-muted font-medium mb-2">Non-FIRE Accounts</p>
      <div class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_52px] bg-elevated px-4 py-2 border-b border-default">
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
          <span />
        </div>
        <div
          v-for="account in nonFireAccounts"
          :key="account.id"
          class="grid grid-cols-[1fr_160px_140px_52px] items-center px-4 py-3 border-b border-default last:border-b-0 cursor-pointer hover:bg-elevated/50 transition-colors"
          @click="navigate(account)"
        >
          <div>
            <p class="text-sm font-medium">{{ account.name }}</p>
            <p v-if="account.institution" class="text-xs text-muted">{{ account.institution }}</p>
          </div>
          <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>
          <div class="text-right">
            <p class="text-sm font-semibold font-mono">{{ latestBalance(account.id) }}</p>
            <p v-if="formatBalanceDate(account.id)" :class="['text-xs', isStaleBalance(account.id) ? 'text-warning' : 'text-muted']">{{ formatBalanceDate(account.id) }}</p>
          </div>
          <div class="flex justify-end" @click.stop>
            <UDropdownMenu :items="activeMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" aria-label="Account options" :loading="busyAccountId === account.id" :disabled="busyAccountId !== null" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Equity -->
    <div v-if="equityAccounts.length > 0" class="mb-8">
      <p class="text-xs text-muted font-medium mb-2">Equity</p>
      <div class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_52px] bg-elevated px-4 py-2 border-b border-default">
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
          <span />
        </div>
        <div
          v-for="account in equityAccounts"
          :key="account.id"
          class="grid grid-cols-[1fr_160px_140px_52px] items-center px-4 py-3 border-b border-default last:border-b-0 cursor-pointer hover:bg-elevated/50 transition-colors"
          @click="navigate(account)"
        >
          <div>
            <p class="text-sm font-medium">{{ account.name }}</p>
            <p v-if="account.institution" class="text-xs text-muted">{{ account.institution }}</p>
          </div>
          <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>
          <div class="text-right">
            <p class="text-sm font-semibold font-mono">{{ latestBalance(account.id) }}</p>
            <p v-if="formatBalanceDate(account.id)" :class="['text-xs', isStaleBalance(account.id) ? 'text-warning' : 'text-muted']">{{ formatBalanceDate(account.id) }}</p>
          </div>
          <div class="flex justify-end" @click.stop>
            <UDropdownMenu :items="activeMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" aria-label="Account options" :loading="busyAccountId === account.id" :disabled="busyAccountId !== null" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Archived -->
    <div v-if="archivedAccounts.length > 0">
      <button
        class="flex items-center gap-1.5 text-xs text-muted font-medium mb-2"
        :aria-expanded="showArchived"
        @click="showArchived = !showArchived"
      >
        <UIcon :name="showArchived ? 'i-ph-caret-down' : 'i-ph-caret-right'" class="w-3.5 h-3.5" />
        <span>Archived ({{ archivedAccounts.length }})</span>
      </button>
      <div v-if="showArchived" class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_52px] bg-elevated px-4 py-2 border-b border-default">
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
          <span />
        </div>
        <div
          v-for="account in archivedAccounts"
          :key="account.id"
          class="grid grid-cols-[1fr_160px_140px_36px] items-center px-4 py-3 border-b border-default last:border-b-0"
        >
          <div>
            <p class="text-sm font-medium text-muted">{{ account.name }}</p>
            <p v-if="account.institution" class="text-xs text-muted">{{ account.institution }}</p>
          </div>
          <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>
          <div class="text-right">
            <p class="text-sm text-muted font-mono">{{ latestBalance(account.id) }}</p>
            <p v-if="formatBalanceDate(account.id)" class="text-xs text-muted">{{ formatBalanceDate(account.id) }}</p>
          </div>
          <div class="flex justify-end">
            <UDropdownMenu :items="archivedMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" aria-label="Account options" :loading="busyAccountId === account.id" :disabled="busyAccountId !== null" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Add / Edit account modal -->
    <UModal
      v-model:open="isAccountModalOpen"
      :title="editingAccount ? 'Edit Account' : 'Add Account'"
      class="w-100"
    >
      <template #body>
        <AccountForm
          :key="editingAccount?.id ?? 'new'"
          :account="editingAccount ?? undefined"
          @saved="onAccountSaved"
        />
      </template>
    </UModal>
  </div>
</template>
