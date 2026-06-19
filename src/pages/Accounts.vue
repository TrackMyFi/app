<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useAccountsStore } from '../stores/accounts'
import { labelForAccountType } from '../lib/accountTypes'
import AccountForm from '../components/AccountForm.vue'
import StatCard from '../components/StatCard.vue'
import type { Account } from '../lib/types/Account'
import { confirm } from '@tauri-apps/plugin-dialog'

const store = useAccountsStore()
const router = useRouter()

onMounted(() => store.loadList())

const fmt = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

const latestBalanceMap = computed(() =>
  new Map(store.latestBalances.map(b => [b.accountId, b.balance]))
)

const activeAccounts = computed(() => store.accounts.filter(a => a.isActive))
const archivedAccounts = computed(() => store.accounts.filter(a => !a.isActive))
const fireAccounts = computed(() => activeAccounts.value.filter(a => a.includeInFireCalculations))
const nonFireAccounts = computed(() => activeAccounts.value.filter(a => !a.includeInFireCalculations))

const netWorth = computed(() =>
  activeAccounts.value.reduce((s, a) => s + (latestBalanceMap.value.get(a.id) ?? 0), 0)
)
const fireTotal = computed(() =>
  fireAccounts.value.reduce((s, a) => s + (latestBalanceMap.value.get(a.id) ?? 0), 0)
)
const nonFireTotal = computed(() =>
  nonFireAccounts.value.reduce((s, a) => s + (latestBalanceMap.value.get(a.id) ?? 0), 0)
)

function latestBalance(accountId: number) {
  const b = latestBalanceMap.value.get(accountId)
  return b != null ? b.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 }) : '—'
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

// Archive / Unarchive
async function archive(id: number) {
  await store.archive(id)
}

async function unarchive(id: number) {
  await store.unarchive(id)
}

// Delete (archived accounts only)
async function remove(account: Account) {
  const ok = await confirm(
    `Permanently delete "${account.name}" and all of its balance snapshots? This cannot be undone.`,
    { title: 'Delete Account?', kind: 'warning' },
  )
  if (ok) await store.remove(account.id)
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
  <div class="p-6 max-w-4xl">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-2xl font-bold">Accounts</h1>
      <UButton icon="i-ph-plus" @click="openAdd">Add Account</UButton>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-3 gap-4 mb-8">
      <StatCard label="Net Worth" :value="fmt(netWorth)" />
      <StatCard label="FIRE Accounts" :value="fmt(fireTotal)" />
      <StatCard label="Non-FIRE Accounts" :value="fmt(nonFireTotal)" />
    </div>

    <!-- FIRE Accounts -->
    <div v-if="fireAccounts.length > 0" class="mb-8">
      <p class="text-xs font-semibold uppercase tracking-widest text-muted mb-2">FIRE Accounts</p>
      <div class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_36px] bg-elevated px-4 py-2 border-b border-default">
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
          <span />
        </div>
        <div
          v-for="account in fireAccounts"
          :key="account.id"
          class="grid grid-cols-[1fr_160px_140px_36px] items-center px-4 py-3 border-b border-default last:border-b-0 cursor-pointer hover:bg-elevated/50 transition-colors"
          @click="navigate(account)"
        >
          <div>
            <p class="text-sm font-medium">{{ account.name }}</p>
            <p v-if="account.institution" class="text-xs text-muted">{{ account.institution }}</p>
          </div>
          <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>
          <span class="text-sm font-semibold text-right font-mono">{{ latestBalance(account.id) }}</span>
          <div class="flex justify-center" @click.stop>
            <UDropdownMenu :items="activeMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Non-FIRE Accounts -->
    <div v-if="nonFireAccounts.length > 0" class="mb-8">
      <p class="text-xs font-semibold uppercase tracking-widest text-muted mb-2">Non-FIRE Accounts</p>
      <div class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_36px] bg-elevated px-4 py-2 border-b border-default">
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
          <span />
        </div>
        <div
          v-for="account in nonFireAccounts"
          :key="account.id"
          class="grid grid-cols-[1fr_160px_140px_36px] items-center px-4 py-3 border-b border-default last:border-b-0 cursor-pointer hover:bg-elevated/50 transition-colors"
          @click="navigate(account)"
        >
          <div>
            <p class="text-sm font-medium">{{ account.name }}</p>
            <p v-if="account.institution" class="text-xs text-muted">{{ account.institution }}</p>
          </div>
          <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>
          <span class="text-sm font-semibold text-right font-mono">{{ latestBalance(account.id) }}</span>
          <div class="flex justify-center" @click.stop>
            <UDropdownMenu :items="activeMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Archived -->
    <div v-if="archivedAccounts.length > 0">
      <button
        class="flex items-center gap-1.5 text-xs font-semibold uppercase tracking-widest text-muted mb-2"
        @click="showArchived = !showArchived"
      >
        <span>{{ showArchived ? '▼' : '▶' }}</span>
        <span>Archived ({{ archivedAccounts.length }})</span>
      </button>
      <div v-if="showArchived" class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_36px] bg-elevated px-4 py-2 border-b border-default">
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
          <span class="text-sm text-muted text-right font-mono">{{ latestBalance(account.id) }}</span>
          <div class="flex justify-center">
            <UDropdownMenu :items="archivedMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Add / Edit account modal -->
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
  </div>
</template>
