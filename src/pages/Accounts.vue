<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useToast } from '@nuxt/ui/composables'
import { useAccountsStore } from '../stores/accounts'
import { groupAccounts, netWorth } from '../lib/accounts/groups'
import AccountForm from '../components/AccountForm.vue'
import AccountsTable from '../components/AccountsTable.vue'
import StatCard from '../components/StatCard.vue'
import type { Account } from '../lib/types/Account'
import { confirm } from '@tauri-apps/plugin-dialog'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'
import { useReveal } from '../composables/useReveal'

const store = useAccountsStore()
const router = useRouter()
const toast = useToast()
const { error, run, retry } = usePageData()

// The summary figures tick up into place the moment balances land — net worth
// is a life target, not a readout, so it should feel like it arrives.
const { progress: reveal, play: playReveal } = useReveal()

onMounted(() => run(async () => {
  await store.loadList()
  await store.load()
  playReveal()
}))

const fmt = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

const latestBalanceMap = computed(() =>
  new Map(store.latestBalances.map(b => [b.accountId, b.balance]))
)
const balanceOf = (id: number) => latestBalanceMap.value.get(id) ?? 0

const activeAccounts = computed(() => store.accounts.filter(a => a.isActive))
const archivedAccounts = computed(() => store.accounts.filter(a => !a.isActive))

// Same categorization shown in the sidebar: Budget accounts are the ones you
// routinely reconcile day to day; Tracking accounts are FIRE accounts you
// just watch grow toward net worth. Equity covers real estate/mortgages.
const groups = computed(() => groupAccounts(store.accounts, balanceOf))
const groupByKey = (key: 'budget' | 'tracking' | 'equity') =>
  computed(() => groups.value.find(g => g.key === key))

const budgetGroup = groupByKey('budget')
const trackingGroup = groupByKey('tracking')
const equityGroup = groupByKey('equity')

const netWorthValue = computed(() => netWorth(store.accounts, balanceOf))

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
    <div :class="['grid gap-4 mb-8', equityGroup ? 'grid-cols-2 lg:grid-cols-4' : 'grid-cols-1 sm:grid-cols-3']">
      <div class="tmfi-rise">
        <StatCard label="Net Worth" :value="fmt(netWorthValue * reveal)" hero />
      </div>
      <div class="tmfi-rise" :style="{ animationDelay: '55ms' }">
        <StatCard label="Budget" hint="Day-to-day accounts" :value="fmt((budgetGroup?.total ?? 0) * reveal)" />
      </div>
      <div class="tmfi-rise" :style="{ animationDelay: '110ms' }">
        <StatCard label="Tracking" hint="FIRE accounts" :value="fmt((trackingGroup?.total ?? 0) * reveal)" />
      </div>
      <div v-if="equityGroup" class="tmfi-rise" :style="{ animationDelay: '165ms' }">
        <StatCard label="Equity" :value="fmt((equityGroup?.total ?? 0) * reveal)" />
      </div>
    </div>

    <!-- Empty state: no accounts at all -->
    <div v-if="store.accounts.length === 0" class="tmfi-rise border border-dashed border-default rounded-lg p-8 text-center mb-8" :style="{ animationDelay: '210ms' }">
      <UIcon name="i-ph-wallet" class="w-8 h-8 text-muted mx-auto mb-3" />
      <p class="text-sm font-medium mb-1">No accounts yet</p>
      <p class="text-sm text-muted">Add your first account above to start tracking your FIRE progress.</p>
    </div>

    <!-- Empty state: all accounts archived -->
    <div v-else-if="activeAccounts.length === 0" class="tmfi-rise border border-dashed border-default rounded-lg p-8 text-center mb-8" :style="{ animationDelay: '210ms' }">
      <UIcon name="i-ph-archive" class="w-8 h-8 text-muted mx-auto mb-3" />
      <p class="text-sm font-medium mb-1">All accounts are archived</p>
      <p class="text-sm text-muted">Restore an account from the list below to see it here.</p>
    </div>

    <!-- Budget Accounts -->
    <AccountsTable
      v-if="budgetGroup"
      class="tmfi-rise block mb-8"
      :style="{ animationDelay: '210ms' }"
      title="Budget Accounts"
      subtitle="Day-to-day spending & cash accounts"
      :accounts="budgetGroup.accounts"
      interactive
      :busy-id="busyAccountId"
      :menu-items="activeMenuItems"
      @select="navigate"
    />

    <!-- Tracking Accounts -->
    <AccountsTable
      v-if="trackingGroup"
      class="tmfi-rise block mb-8"
      :style="{ animationDelay: '260ms' }"
      title="Tracking Accounts"
      subtitle="FIRE accounts — long-term growth, not routinely budgeted"
      :accounts="trackingGroup.accounts"
      interactive
      :busy-id="busyAccountId"
      :menu-items="activeMenuItems"
      @select="navigate"
    />

    <!-- Equity -->
    <AccountsTable
      v-if="equityGroup"
      class="tmfi-rise block mb-8"
      :style="{ animationDelay: '310ms' }"
      title="Equity"
      subtitle="Real estate & mortgages"
      :accounts="equityGroup.accounts"
      interactive
      :busy-id="busyAccountId"
      :menu-items="activeMenuItems"
      @select="navigate"
    />

    <!-- Archived -->
    <div v-if="archivedAccounts.length > 0">
      <button
        class="flex items-center gap-1.5 text-xs text-muted font-medium mb-2 rounded hover:text-default transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30"
        :aria-expanded="showArchived"
        @click="showArchived = !showArchived"
      >
        <UIcon :name="showArchived ? 'i-ph-caret-down' : 'i-ph-caret-right'" class="w-3.5 h-3.5" />
        <span>Archived ({{ archivedAccounts.length }})</span>
      </button>
      <AccountsTable
        v-if="showArchived"
        title="Archived"
        class="[&_.account-table-heading]:sr-only"
        :accounts="archivedAccounts"
        :busy-id="busyAccountId"
        :menu-items="archivedMenuItems"
      />
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
