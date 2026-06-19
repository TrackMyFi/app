<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { DateTime } from 'luxon'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useAccountsStore } from '../stores/accounts'
import { labelForAccountType } from '../lib/accountTypes'
import * as api from '../lib/api/accounts'
import type { AccountBalance } from '../lib/types/AccountBalance'
import type { BalanceMonthSummary } from '../lib/types/BalanceMonthSummary'
import AccountBalanceChart from '../components/AccountBalanceChart.vue'
import type { ChartPoint } from '../components/AccountBalanceChart.vue'
import AccountForm from '../components/AccountForm.vue'
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

const openMonth = ref<string | null>(null)
const monthCache = ref(new Map<string, AccountBalance[]>())

async function toggleMonth(month: string) {
  if (openMonth.value === month) {
    openMonth.value = null
    return
  }
  if (!monthCache.value.has(month)) {
    const rows = await api.listBalancesForMonth(accountId.value, month)
    monthCache.value.set(month, rows)
  }
  openMonth.value = month
}

function cachedRows(month: string): AccountBalance[] {
  return monthCache.value.get(month) ?? []
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

const fmtDate = (s: string) => DateTime.fromISO(s).toFormat('MMM d, yyyy')
const fmtMoney = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
const fmtMonthLabel = (s: string) => DateTime.fromISO(s + '-01').toFormat('MMMM yyyy')

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
const txnForModal = ref<number | null>(null)

// We only have the linkedTransactionId, not the full Transaction object.
// Show a minimal modal with just the linked txn ID for now.
function openTxnModal(balance: AccountBalance) {
  txnForModal.value = balance.linkedTransactionId
  txnModalOpen.value = true
}

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
  <div v-if="account" class="p-6 max-w-3xl">
    <!-- Back link -->
    <button
      class="text-sm text-primary hover:underline mb-4 flex items-center gap-1"
      @click="router.push({ name: 'accounts' })"
    >
      ← Accounts
    </button>

    <!-- Header -->
    <div class="flex items-start justify-between mb-6">
      <div>
        <h1 class="text-2xl font-bold">{{ account.name }}</h1>
        <p class="text-sm text-muted mt-0.5">
          {{ labelForAccountType(account.type) }}
          <template v-if="account.institution"> · {{ account.institution }}</template>
          <template v-if="account.includeInFireCalculations"> · FIRE ✓</template>
        </p>
      </div>
      <div class="flex gap-2">
        <UButton size="sm" @click="editAccountModalOpen = true">Edit</UButton>
        <UButton size="sm" variant="ghost" color="neutral" @click="archiveAccount">Archive</UButton>
      </div>
    </div>

    <!-- Chart -->
    <div class="border border-default rounded-lg p-4 mb-6">
      <p class="text-xs font-semibold uppercase tracking-widest text-muted mb-3">{{ chartTitle }}</p>
      <AccountBalanceChart v-if="chartPoints.length > 0" :points="chartPoints" :mode="chartMode" />
      <p v-else class="text-sm text-muted text-center py-8">No snapshots yet</p>
    </div>

    <!-- Accordion header -->
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-sm font-semibold">Balance Snapshots</h2>
      <UButton size="sm" icon="i-ph-plus" @click="addModalOpen = true">Add Snapshot</UButton>
    </div>

    <!-- Month accordion -->
    <div class="border border-default rounded-lg overflow-hidden mb-8">
      <div v-if="monthSummaries.length === 0" class="px-4 py-8 text-center text-sm text-muted">
        No snapshots yet
      </div>
      <div
        v-for="(summary, idx) in monthSummaries"
        :key="summary.month"
        :class="['border-default', idx < monthSummaries.length - 1 ? 'border-b' : '']"
      >
        <!-- Month header row -->
        <button
          class="w-full flex items-center justify-between px-4 py-3 text-left hover:bg-elevated/50 transition-colors"
          @click="toggleMonth(summary.month)"
        >
          <span class="text-sm font-medium">
            {{ openMonth === summary.month ? '▼' : '▶' }} {{ fmtMonthLabel(summary.month) }}
          </span>
          <span class="text-xs text-muted">
            {{ summary.count }} snapshot{{ summary.count !== 1 ? 's' : '' }} · {{ fmtMoney(summary.latestBalance) }}
          </span>
        </button>

        <!-- Expanded rows -->
        <div v-if="openMonth === summary.month" class="bg-elevated/30">
          <div
            v-for="row in cachedRows(summary.month)"
            :key="row.id"
            class="grid grid-cols-[120px_1fr_100px] items-center px-6 py-2 border-t border-default"
          >
            <!-- Normal row -->
            <template v-if="editingSnapshotId !== row.id">
              <span class="text-xs text-muted">{{ fmtDate(row.recordedAt) }}</span>
              <span class="text-sm font-semibold">{{ fmtMoney(row.balance) }}</span>
              <div class="flex items-center justify-end gap-1">
                <UButton
                  v-if="row.linkedTransactionId != null"
                  size="xs"
                  variant="ghost"
                  color="neutral"
                  icon="i-ph-receipt"
                  title="Linked transaction"
                  @click="openTxnModal(row)"
                />
                <UButton
                  size="xs"
                  variant="ghost"
                  color="neutral"
                  icon="i-ph-pencil"
                  @click="startEdit(row)"
                />
                <UButton
                  size="xs"
                  variant="ghost"
                  color="error"
                  icon="i-ph-trash"
                  @click="deleteSnapshot(row)"
                />
              </div>
            </template>

            <!-- Inline edit row -->
            <template v-else>
              <DateInput v-model="editDate" class="text-xs" />
              <CurrencyInput v-model="editBalance" class="text-sm mx-2" />
              <div class="flex items-center justify-end gap-1">
                <UButton size="xs" @click="saveEdit(row)">Save</UButton>
                <UButton size="xs" variant="ghost" color="neutral" @click="cancelEdit">Cancel</UButton>
              </div>
            </template>
          </div>
        </div>
      </div>
    </div>

    <!-- Danger zone -->
    <div class="border border-error/30 rounded-lg p-4">
      <h3 class="text-sm font-semibold text-error mb-1">Danger Zone</h3>
      <p class="text-xs text-muted mb-3">Permanently deletes this account and all of its balance snapshots. Cannot be undone.</p>
      <UButton color="error" variant="outline" size="sm" @click="deleteAccount">Delete Account</UButton>
    </div>

    <!-- Edit account modal -->
    <UModal v-model:open="editAccountModalOpen" title="Edit Account" class="w-112">
      <template #body>
        <AccountForm :key="account.id" :account="account" @saved="onAccountSaved" />
      </template>
    </UModal>

    <!-- Add snapshot modal -->
    <UModal v-model:open="addModalOpen" title="Add Snapshot" class="w-96">
      <template #body>
        <div class="space-y-4">
          <UFormField label="Balance">
            <CurrencyInput v-model="newBalance" class="w-full" />
          </UFormField>
          <UFormField label="Date">
            <DateInput v-model="newDate" />
          </UFormField>
          <div class="flex justify-end pt-2">
            <UButton @click="submitAddSnapshot">Add Snapshot</UButton>
          </div>
        </div>
      </template>
    </UModal>

    <!-- Linked transaction modal (minimal — just shows transaction ID) -->
    <UModal v-model:open="txnModalOpen" title="Linked Transaction">
      <template #body>
        <p class="text-sm text-muted">Transaction #{{ txnForModal }}</p>
      </template>
    </UModal>
  </div>

  <!-- Account not found -->
  <div v-else class="p-6 text-muted text-sm">Account not found.</div>
</template>
