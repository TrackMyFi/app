<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { DateTime } from 'luxon'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useAccountsStore } from '../stores/accounts'
import { labelForAccountType } from '../lib/accountTypes'
import * as api from '../lib/api/accounts'
import { getTransaction } from '../lib/api/transactions'
import type { AccountBalance } from '../lib/types/AccountBalance'
import type { BalanceMonthSummary } from '../lib/types/BalanceMonthSummary'
import type { Transaction } from '../lib/types/Transaction'
import AccountBalanceChart from '../components/AccountBalanceChart.vue'
import type { ChartPoint } from '../components/AccountBalanceChart.vue'
import AccountForm from '../components/AccountForm.vue'
import StatCard from '../components/StatCard.vue'
import CurrencyInput from '../components/CurrencyInput.vue'
import DateInput from '../components/DateInput.vue'

const route = useRoute()
const router = useRouter()
const store = useAccountsStore()

const accountId = computed(() => Number(route.params.id))
const account = computed(() => store.accounts.find(a => a.id === accountId.value) ?? null)

// Load accounts if store is empty (direct navigation)
onMounted(async () => {
  if (store.accounts.length === 0) await store.loadList()
  await refreshSummaries()
})

// ─── Month summaries ────────────────────────────────────────────────────────

const monthSummaries = ref<BalanceMonthSummary[]>([])

async function refreshSummaries() {
  monthSummaries.value = await api.listBalanceMonthSummaries(accountId.value)
}

// ─── Accordion ──────────────────────────────────────────────────────────────

const openMonthValue = ref<string | undefined>(undefined)
const openMonth = computed(() => openMonthValue.value ?? null)
const monthCache = ref(new Map<string, AccountBalance[]>())

watch(openMonthValue, async (month) => {
  if (month && !monthCache.value.has(month)) {
    const rows = await api.listBalancesForMonth(accountId.value, month)
    monthCache.value.set(month, rows)
  }
})

const accordionItems = computed(() =>
  monthSummaries.value.map((s, i) => {
    const prev = monthSummaries.value[i + 1]
    const delta = prev != null ? s.latestBalance - prev.latestBalance : null
    const deltaPercent = (delta != null && prev != null && Math.abs(prev.latestBalance) >= 0.01)
      ? (delta / Math.abs(prev.latestBalance)) * 100
      : null
    return {
      label: fmtMonthLabel(s.month),
      value: s.month,
      count: s.count,
      latestBalance: s.latestBalance,
      delta: delta !== 0 ? delta : null,
      deltaPercent,
    }
  })
)

function cachedRows(month: string): AccountBalance[] {
  return monthCache.value.get(month) ?? []
}

function getSnapshotDelta(month: string, rowId: number): { delta: number; pct: number | null } | null {
  const rows = cachedRows(month)
  const idx = rows.findIndex(r => r.id === rowId)
  if (idx === -1 || idx === rows.length - 1) return null
  const prev = rows[idx + 1]
  const delta = rows[idx].balance - prev.balance
  if (delta === 0) return null
  const pct = Math.abs(prev.balance) >= 0.01 ? (delta / Math.abs(prev.balance)) * 100 : null
  return { delta, pct }
}

async function invalidateMonth(month: string) {
  monthCache.value.delete(month)
  if (openMonth.value === month) {
    const rows = await api.listBalancesForMonth(accountId.value, month)
    monthCache.value.set(month, rows)
  }
}

// ─── Chart ──────────────────────────────────────────────────────────────────

const chartMode = computed<'monthly' | 'intramonth'>(() =>
  openMonth.value ? 'intramonth' : 'monthly'
)

const chartPoints = computed<ChartPoint[]>(() => {
  if (openMonth.value) {
    const rows = cachedRows(openMonth.value)
    return [...rows].reverse().map(r => ({ date: r.recordedAt, balance: r.balance }))
  }
  return [...monthSummaries.value].reverse().map(s => ({ date: s.month + '-01', balance: s.latestBalance }))
})

const chartTitle = computed(() =>
  openMonth.value
    ? DateTime.fromISO(openMonth.value + '-01').toFormat('MMMM yyyy') + ' — Snapshot Detail'
    : 'Balance History — Monthly'
)

// ─── Snapshot formatting ─────────────────────────────────────────────────────

const fmtDate = (s: string) => DateTime.fromISO(s).toFormat('MMM dd')
const fmtMoney = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
const fmtMonthLabel = (s: string) => DateTime.fromISO(s + '-01').toFormat('MMM yyyy')
const fmtDelta = (delta: number) => (delta > 0 ? '+' : '') + delta.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
const fmtPercent = (pct: number | null) => pct == null ? '' : (pct > 0 ? '+' : '') + pct.toFixed(1) + '%'

const currentBalance = computed(() => monthSummaries.value[0]?.latestBalance ?? null)
const currentBalanceAsOf = computed(() => monthSummaries.value[0] ? fmtMonthLabel(monthSummaries.value[0].month) : '')
const headerMenuItems = computed(() => [[
  { label: 'Archive', icon: 'i-ph-archive', onSelect: archiveAccount },
]])

// ─── Inline snapshot editing ─────────────────────────────────────────────────

const editingSnapshotId = ref<number | null>(null)
const editBalance = ref<number>(0)
const editDate = ref<string>('')

function startEdit(row: AccountBalance) {
  editingSnapshotId.value = row.id
  editBalance.value = row.balance
  editDate.value = row.recordedAt
}

function cancelEdit() {
  editingSnapshotId.value = null
}

async function saveEdit(row: AccountBalance) {
  await api.updateBalance({ id: row.id, balance: editBalance.value, recordedAt: editDate.value })
  const month = editDate.value.slice(0, 7)
  const oldMonth = row.recordedAt.slice(0, 7)
  await refreshSummaries()
  await invalidateMonth(month)
  if (oldMonth !== month) await invalidateMonth(oldMonth)
  editingSnapshotId.value = null
}

async function deleteSnapshot(row: AccountBalance) {
  const ok = await confirm(
    `Delete this snapshot (${fmtDate(row.recordedAt)} · ${fmtMoney(row.balance)})? This cannot be undone.`,
    { title: 'Delete Snapshot?', kind: 'warning' },
  )
  if (!ok) return
  await api.deleteBalance(row.id)
  const month = row.recordedAt.slice(0, 7)
  await refreshSummaries()
  await invalidateMonth(month)
}

// ─── Linked transaction modal ─────────────────────────────────────────────────

const txnModalOpen = ref(false)
const txnData = ref<Transaction | null>(null)
const txnLoading = ref(false)

async function openTxnModal(balance: AccountBalance) {
  txnData.value = null
  txnModalOpen.value = true
  if (balance.linkedTransactionId != null) {
    txnLoading.value = true
    try {
      txnData.value = await getTransaction(balance.linkedTransactionId)
    } finally {
      txnLoading.value = false
    }
  }
}

watch(txnModalOpen, open => {
  if (!open) txnData.value = null
})

// ─── Add snapshot modal ───────────────────────────────────────────────────────

const addModalOpen = ref(false)
const newBalance = ref<number>(0)
const newDate = ref<string>(DateTime.now().toISODate()!)

async function submitAddSnapshot() {
  const savedMonth = newDate.value.slice(0, 7)
  await api.addBalance({ accountId: accountId.value, balance: newBalance.value, recordedAt: newDate.value })
  addModalOpen.value = false
  newBalance.value = 0
  newDate.value = DateTime.now().toISODate()!
  await refreshSummaries()
  await invalidateMonth(savedMonth)
}

watch(addModalOpen, open => {
  if (!open) {
    newBalance.value = 0
    newDate.value = DateTime.now().toISODate()!
  }
})

// ─── Edit account modal ───────────────────────────────────────────────────────

const editAccountModalOpen = ref(false)

function onAccountSaved() {
  editAccountModalOpen.value = false
  store.loadList()
}

// ─── Archive / Delete account ─────────────────────────────────────────────────

async function archiveAccount() {
  if (!account.value) return
  const ok = await confirm(
    `Archive "${account.value.name}"? It will be hidden from the accounts list but its data will be preserved.`,
    { title: 'Archive Account?', kind: 'warning' },
  )
  if (!ok) return
  await store.archive(account.value.id)
  router.push({ name: 'accounts' })
}

async function deleteAccount() {
  if (!account.value) return
  const ok = await confirm(
    `Permanently delete "${account.value.name}" and ALL of its balance snapshots? This cannot be undone.`,
    { title: 'Delete Account?', kind: 'warning' },
  )
  if (!ok) return
  await store.remove(account.value.id)
  router.push({ name: 'accounts' })
}
</script>

<template>
  <div v-if="account" class="p-6">
    <!-- Back link -->
    <RouterLink
      :to="{ name: 'accounts' }"
      class="text-sm text-primary hover:underline mb-4 flex items-center gap-1"
    >
      <UIcon name="i-ph-arrow-left" class="w-4 h-4" />
      Accounts
    </RouterLink>

    <!-- Header -->
    <div class="flex items-start justify-between mb-4">
      <div>
        <h1 class="text-2xl font-bold">{{ account.name }}</h1>
        <p class="text-sm text-muted mt-0.5">
          {{ labelForAccountType(account.type) }}
          <template v-if="account.institution"> · {{ account.institution }}</template>
          <template v-if="account.includeInFireCalculations"> · FIRE <UIcon name="i-ph-check-circle" class="w-3.5 h-3.5 text-success inline-block align-text-bottom" aria-hidden="true" /></template>
        </p>
      </div>
      <div class="flex gap-2">
        <UButton @click="editAccountModalOpen = true">Edit</UButton>
        <UDropdownMenu :items="headerMenuItems">
          <UButton variant="ghost" color="neutral" icon="i-ph-dots-three" aria-label="More options" />
        </UDropdownMenu>
      </div>
    </div>

    <!-- Current balance stat -->
    <StatCard
      v-if="currentBalance !== null"
      label="Current Balance"
      :value="fmtMoney(currentBalance)"
      :hint="`as of ${currentBalanceAsOf}`"
      class="mb-6"
    />

    <!-- Chart -->
    <div class="border border-default rounded-lg p-4 mb-6">
      <p class="text-xs font-semibold uppercase tracking-widest text-muted mb-3 transition-opacity duration-200">{{ chartTitle }}</p>
      <AccountBalanceChart v-if="chartPoints.length > 0" :points="chartPoints" :mode="chartMode" />
      <p v-else class="text-sm text-muted text-center py-8">No snapshots yet</p>
      <p v-if="chartMode === 'monthly' && chartPoints.length > 0" class="text-xs text-muted mt-2">Open a month below to see daily snapshot detail.</p>
    </div>

    <!-- Accordion header -->
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-sm font-semibold">Balance Snapshots</h2>
      <UButton size="sm" icon="i-ph-plus" @click="addModalOpen = true">Add Snapshot</UButton>
    </div>

    <!-- Month accordion -->
    <div class="mb-8">
      <div v-if="monthSummaries.length === 0" class="border border-default rounded-lg px-4 py-8 text-center text-sm text-muted">
        No snapshots yet
      </div>
      <UAccordion
        v-else
        v-model="openMonthValue"
        :items="accordionItems"
        class="u-accordion border border-default rounded-lg"
        collapsible
      >
        <template #default="{ item }">
          <div class="w-18">{{ item.label }}</div>
        </template>
        <template #trailing="{ item, open }">
          <div class="flex-1 flex items-center gap-3 mx-3">
            <span class="text-xs text-muted mr-auto">
              {{ item.count }} snapshot{{ item.count !== 1 ? 's' : '' }}
            </span>
            <UBadge v-if="item.delta != null" :icon="item.delta > 0 ? 'i-ph-trend-up' : 'i-ph-trend-down'" :color="item.delta > 0 ? 'success' : 'error'" variant="soft" size="sm">
              {{ fmtDelta(item.delta) }}
              <template v-if="item.deltaPercent != null">&nbsp;({{ fmtPercent(item.deltaPercent) }})</template>
            </UBadge>
            <span class="w-20 text-end font-mono">
              {{ fmtMoney(item.latestBalance) }}
            </span>
          </div>
          <UIcon
            name="i-ph-caret-down"
            class="w-4 h-4 shrink-0 transition-transform duration-200"
            :class="open ? 'rotate-180' : ''"
          />
        </template>
        <template #body="{ item }">
          <div
            v-for="row in cachedRows(item.value)"
            :key="row.id"
            class="grid items-center py-1.5 not-first:border-t border-default"
            :class="editingSnapshotId === row.id ? 'grid-cols-[75px_1fr_100px]' : 'grid-cols-[75px_1fr_auto_100px]'"
          >
            <template v-if="editingSnapshotId !== row.id">
              <span class="text-xs text-muted">{{ fmtDate(row.recordedAt) }}</span>
              <span class="text-sm font-semibold font-mono">{{ fmtMoney(row.balance) }}</span>
              <template v-for="d in [getSnapshotDelta(item.value, row.id)]">
                <UBadge v-if="d" :icon="d.delta > 0 ? 'i-ph-trend-up' : 'i-ph-trend-down'" :color="d.delta > 0 ? 'success' : 'error'" variant="soft" size="sm" class="mr-6">
                  {{ fmtDelta(d.delta) }}
                  <template v-if="d.pct != null">&nbsp;({{ fmtPercent(d.pct) }})</template>
                </UBadge>
                <span v-else />
              </template>
              <div class="flex items-center justify-end gap-1">
                <UButton v-if="row.linkedTransactionId != null" size="xs" variant="ghost" color="neutral" icon="i-ph-receipt" aria-label="View linked transaction" @click="openTxnModal(row)" />
                <UButton size="xs" variant="ghost" color="neutral" icon="i-ph-pencil" aria-label="Edit snapshot" @click="startEdit(row)" />
                <UButton size="xs" variant="ghost" color="error" icon="i-ph-trash" aria-label="Delete snapshot" @click="deleteSnapshot(row)" />
              </div>
            </template>
            <template v-else>
              <DateInput v-model="editDate" class="text-xs" />
              <CurrencyInput v-model="editBalance" class="text-sm mx-2" />
              <div class="flex items-center justify-end gap-1">
                <UButton size="xs" @click="saveEdit(row)">Save</UButton>
                <UButton size="xs" variant="ghost" color="neutral" @click="cancelEdit">Cancel</UButton>
              </div>
            </template>
          </div>
        </template>
      </UAccordion>
    </div>

    <!-- Danger zone -->
    <div v-if="!account.isActive" class="border border-error/30 rounded-lg p-4">
      <h3 class="text-sm font-semibold text-error mb-1">Danger Zone</h3>
      <p class="text-xs text-muted mb-3">Permanently deletes this account and all of its balance snapshots. Cannot be undone.</p>
      <UButton color="error" variant="outline" size="sm" @click="deleteAccount">Delete Account</UButton>
    </div>

    <!-- Edit account modal -->
    <UModal v-model:open="editAccountModalOpen" title="Edit Account" class="w-100">
      <template #body>
        <AccountForm :key="account.id" :account="account" @saved="onAccountSaved" />
      </template>
    </UModal>

    <!-- Add snapshot modal -->
    <UModal v-model:open="addModalOpen" title="Add Snapshot" class="w-84">
      <template #body>
        <div class="space-y-4">
          <UFormField label="Balance">
            <CurrencyInput v-model="newBalance" class="w-full" />
          </UFormField>
          <UFormField label="Date">
            <DateInput v-model="newDate" class="w-full" />
          </UFormField>
          <div class="flex justify-end pt-2">
            <UButton @click="submitAddSnapshot" block>Add Snapshot</UButton>
          </div>
        </div>
      </template>
    </UModal>

    <!-- Linked transaction modal -->
    <UModal v-model:open="txnModalOpen" title="Linked Transaction">
      <template #body>
        <div v-if="txnLoading" class="text-sm text-muted text-center py-4">Loading…</div>
        <div v-else-if="txnData" class="space-y-3">
          <div class="flex justify-between items-baseline gap-4">
            <span class="text-xs text-muted">Amount</span>
            <span :class="['text-sm font-semibold font-mono', txnData.amount >= 0 ? 'text-success' : 'text-error']">{{ fmtMoney(txnData.amount) }}</span>
          </div>
          <div class="flex justify-between items-baseline gap-4">
            <span class="text-xs text-muted">Date</span>
            <span class="text-sm">{{ DateTime.fromISO(txnData.date).toFormat('MMM d, yyyy') }}</span>
          </div>
          <div v-if="txnData.description" class="flex justify-between items-start gap-4">
            <span class="text-xs text-muted shrink-0">Description</span>
            <span class="text-sm text-right">{{ txnData.description }}</span>
          </div>
          <div v-if="txnData.category" class="flex justify-between items-baseline gap-4">
            <span class="text-xs text-muted">Category</span>
            <span class="text-sm capitalize">{{ txnData.category }}</span>
          </div>
          <div class="flex justify-between items-baseline gap-4">
            <span class="text-xs text-muted">Type</span>
            <span class="text-sm capitalize">{{ txnData.type }}</span>
          </div>
        </div>
        <p v-else class="text-sm text-muted text-center py-4">Transaction not found.</p>
      </template>
    </UModal>
  </div>

  <!-- Account not found -->
  <div v-else class="p-6 text-muted text-sm">Account not found.</div>
</template>