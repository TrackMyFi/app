<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { DateTime } from 'luxon'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useToast } from '@nuxt/ui/composables'
import { useAccountsStore } from '../stores/accounts'
import { labelForAccountType } from '../lib/accountTypes'
import * as api from '../lib/api/accounts'
import { getTransaction } from '../lib/api/transactions'
import { listAssetEvents, deleteAssetEvent } from '../lib/api/assetEvents'
import { costBasisAdded, upkeepCost } from '../lib/assets/rollups'
import { labelForAssetEventKind, colorForAssetEventKind } from '../lib/assets/constants'
import { crossedMilestone } from '../lib/balances/milestones'
import { useCountUp } from '../composables/useCountUp'
import type { AccountBalance } from '../lib/types/AccountBalance'
import type { BalanceMonthSummary } from '../lib/types/BalanceMonthSummary'
import type { Transaction } from '../lib/types/Transaction'
import type { AssetEvent } from '../lib/types/AssetEvent'
import AccountBalanceChart from '../components/AccountBalanceChart.vue'
import type { ChartPoint } from '../components/AccountBalanceChart.vue'
import AccountForm from '../components/AccountForm.vue'
import AssetEventForm from '../components/AssetEventForm.vue'
import StatCard from '../components/StatCard.vue'
import CurrencyInput from '../components/CurrencyInput.vue'
import DateInput from '../components/DateInput.vue'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'

const route = useRoute()
const router = useRouter()
const store = useAccountsStore()
const toast = useToast()
const { error, run, retry } = usePageData()

const accountId = computed(() => Number(route.params.id))
const account = computed(() => store.accounts.find(a => a.id === accountId.value) ?? null)

// Load accounts if store is empty (direct navigation)
onMounted(() => run(async () => {
  if (store.accounts.length === 0) await store.loadList()
  await refreshSummaries()
  await loadAssetEvents()
}))

// ─── Upkeep & improvements (real estate only) ────────────────────────────────

const isRealEstate = computed(() => account.value?.type === 'real_estate')
const assetEvents = ref<AssetEvent[]>([])
const assetEventModalOpen = ref(false)
const editingAssetEvent = ref<AssetEvent | null>(null)
const deletingAssetEventId = ref<number | null>(null)

async function loadAssetEvents() {
  if (!isRealEstate.value) { assetEvents.value = []; return }
  assetEvents.value = await listAssetEvents({ accountId: accountId.value })
}

const basisAdded = computed(() => costBasisAdded(assetEvents.value))
const lifetimeUpkeep = computed(() => upkeepCost(assetEvents.value))

function openAddAssetEvent() { editingAssetEvent.value = null; assetEventModalOpen.value = true }
function openEditAssetEvent(e: AssetEvent) { editingAssetEvent.value = e; assetEventModalOpen.value = true }

async function onAssetEventSaved() {
  assetEventModalOpen.value = false
  await loadAssetEvents()
}

watch(assetEventModalOpen, open => { if (!open) editingAssetEvent.value = null })

async function removeAssetEvent(e: AssetEvent) {
  const ok = await confirm(`Delete "${e.description}" on ${e.date}?`, { title: 'Delete asset event', kind: 'warning' })
  if (!ok) return
  deletingAssetEventId.value = e.id
  try {
    await deleteAssetEvent(e.id)
    await loadAssetEvents()
  } catch (err) {
    toast.add({ title: 'Failed to delete event', description: String(err), color: 'error' })
  } finally {
    deletingAssetEventId.value = null
  }
}

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

// ─── Headline balance: count-up + milestone celebration ───────────────────────
// The current balance is the number this page exists for. When it grows after
// an edit it rolls up to land with weight; when it crosses a major milestone
// ($10k, $100k, the first million…) a single emerald burst marks the moment.

const { value: displayBalance, set: setBalance, animateTo: animateBalance } = useCountUp(0)
const fmtMilestone = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

const milestoneCelebrating = ref(false)
const moteKey = ref(0)
let milestoneTimer: ReturnType<typeof setTimeout> | undefined
const MOTE_COUNT = 7

function moteStyle(i: number) {
  const spread = 44 // % of the card width the motes fan across
  const left = 50 - spread / 2 + (spread * (i - 1)) / (MOTE_COUNT - 1)
  return { left: `${left}%`, animationDelay: `${(i * 53) % 240}ms` }
}

function celebrateMilestone(amount: number) {
  toast.add({
    title: `${fmtMilestone(amount)} milestone`,
    description: `${account.value?.name ?? 'This account'} just crossed ${fmtMilestone(amount)}.`,
    color: 'success',
    icon: 'i-ph-confetti',
  })
  moteKey.value++
  milestoneCelebrating.value = true
  clearTimeout(milestoneTimer)
  milestoneTimer = setTimeout(() => { milestoneCelebrating.value = false }, 1700)
}

let balanceInitialized = false
watch(currentBalance, (next, prev) => {
  if (next == null) return
  if (!balanceInitialized) {
    balanceInitialized = true
    setBalance(next) // first paint settles quietly — no page-load theatrics
    return
  }
  if (prev != null && next > prev) {
    animateBalance(next)
    const milestone = crossedMilestone(prev, next)
    if (milestone != null) celebrateMilestone(milestone)
  } else {
    setBalance(next) // corrections and drops update without fanfare
  }
})

onUnmounted(() => clearTimeout(milestoneTimer))

// ─── Inline snapshot editing ─────────────────────────────────────────────────

const editingSnapshotId = ref<number | null>(null)
const editBalance = ref<number>(0)
const editDate = ref<string>('')
const savingSnapshotId = ref<number | null>(null)
const deletingSnapshotId = ref<number | null>(null)

function startEdit(row: AccountBalance) {
  editingSnapshotId.value = row.id
  editBalance.value = row.balance
  editDate.value = row.recordedAt
}

function cancelEdit() {
  editingSnapshotId.value = null
}

async function saveEdit(row: AccountBalance) {
  savingSnapshotId.value = row.id
  try {
    await api.updateBalance({ id: row.id, balance: editBalance.value, recordedAt: editDate.value })
    const month = editDate.value.slice(0, 7)
    const oldMonth = row.recordedAt.slice(0, 7)
    await refreshSummaries()
    await invalidateMonth(month)
    if (oldMonth !== month) await invalidateMonth(oldMonth)
    editingSnapshotId.value = null
  } catch (err) {
    toast.add({ title: 'Failed to update snapshot', description: String(err), color: 'error' })
  } finally {
    savingSnapshotId.value = null
  }
}

async function deleteSnapshot(row: AccountBalance) {
  const ok = await confirm(
    `Delete this snapshot (${fmtDate(row.recordedAt)} · ${fmtMoney(row.balance)})? This cannot be undone.`,
    { title: 'Delete Snapshot?', kind: 'warning' },
  )
  if (!ok) return
  deletingSnapshotId.value = row.id
  try {
    await api.deleteBalance(row.id)
    const month = row.recordedAt.slice(0, 7)
    await refreshSummaries()
    await invalidateMonth(month)
  } catch (err) {
    toast.add({ title: 'Failed to delete snapshot', description: String(err), color: 'error' })
  } finally {
    deletingSnapshotId.value = null
  }
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

// ─── Rebuild snapshots ────────────────────────────────────────────────────────

const rebuildingSnapshots = ref(false)

async function rebuildSnapshots() {
  const ok = await confirm(
    'Recalculate all transaction-linked snapshots using manual snapshots as anchors? Manual snapshots will not be changed.',
    { title: 'Rebuild Snapshots?', kind: 'warning' },
  )
  if (!ok) return
  rebuildingSnapshots.value = true
  try {
    await api.rebuildAccountBalances(accountId.value)
    monthCache.value.clear()
    await refreshSummaries()
    if (openMonthValue.value) {
      const rows = await api.listBalancesForMonth(accountId.value, openMonthValue.value)
      monthCache.value.set(openMonthValue.value, rows)
    }
    toast.add({ title: 'Snapshots rebuilt', color: 'success' })
  } catch (err) {
    toast.add({ title: 'Failed to rebuild snapshots', description: String(err), color: 'error' })
  } finally {
    rebuildingSnapshots.value = false
  }
}

// ─── Add snapshot modal ───────────────────────────────────────────────────────

const addModalOpen = ref(false)
const newBalance = ref<number>(0)
const newDate = ref<string>(DateTime.now().toISODate()!)
const addingSnapshot = ref(false)
const addForm = ref<HTMLFormElement | null>(null)

// Focus the first field once the modal's open transition settles — running
// after `after:enter` means Reka's own focus handling has already fired, so
// ours wins. The date field's first segment is a spinbutton, not an <input>.
function focusFirstField() {
  const el = addForm.value?.querySelector<HTMLElement>(
    'input, [role="spinbutton"], [tabindex]:not([tabindex="-1"])',
  )
  el?.focus()
}

// ─── Save modes (split button) ───────────────────────────────────────────────
// 'close' — save and dismiss the modal (default)
// 'add'   — save, reset to a blank form, keep the modal open for the next entry
// 'keep'  — save, keep the current values, keep the modal open
type SaveMode = 'close' | 'add' | 'keep'

const saveModeOptions: { value: SaveMode; label: string }[] = [
  { value: 'close', label: 'Save Snapshot' },
  { value: 'add', label: 'Save & Add Another' },
  { value: 'keep', label: 'Save, Keep Values, & Add Another' },
]

const SAVE_MODE_KEY = 'trackmyfi.snapshotSaveMode'
function loadSaveMode(): SaveMode {
  const v = localStorage.getItem(SAVE_MODE_KEY)
  return v === 'add' || v === 'keep' ? v : 'close'
}
const saveMode = ref<SaveMode>(loadSaveMode())
watch(saveMode, (m) => localStorage.setItem(SAVE_MODE_KEY, m))

const saveModeLabel = computed(
  () => saveModeOptions.find((o) => o.value === saveMode.value)?.label ?? 'Save Snapshot',
)
const saveMenuItems = computed(() => [
  saveModeOptions.map((o) => ({
    label: o.label,
    icon: saveMode.value === o.value ? 'i-ph-check' : undefined,
    onSelect: () => {
      saveMode.value = o.value
      submitAddSnapshot(o.value)
    },
  })),
])

async function submitAddSnapshot(mode: SaveMode = 'close') {
  const savedMonth = newDate.value.slice(0, 7)
  addingSnapshot.value = true
  try {
    await api.addBalance({ accountId: accountId.value, balance: newBalance.value, recordedAt: newDate.value })
    toast.add({ title: 'Snapshot added', color: 'success' })
    // 'add' starts fresh; 'keep' leaves the entered values in place for the next entry.
    if (mode === 'close') {
      addModalOpen.value = false
    } else if (mode === 'add') {
      newBalance.value = 0
      newDate.value = DateTime.now().toISODate()!
    }
    await refreshSummaries()
    await invalidateMonth(savedMonth)
  } catch (err) {
    toast.add({ title: 'Failed to add snapshot', description: String(err), color: 'error' })
  } finally {
    addingSnapshot.value = false
  }
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

const accountActionBusy = ref(false)

async function archiveAccount() {
  if (!account.value) return
  const ok = await confirm(
    `Archive "${account.value.name}"? It will be hidden from the accounts list but its data will be preserved.`,
    { title: 'Archive Account?', kind: 'warning' },
  )
  if (!ok) return
  accountActionBusy.value = true
  try {
    await store.archive(account.value.id)
    router.push({ name: 'accounts' })
  } catch (err) {
    toast.add({ title: 'Failed to archive account', description: String(err), color: 'error' })
  } finally {
    accountActionBusy.value = false
  }
}

async function deleteAccount() {
  if (!account.value) return
  const ok = await confirm(
    `Permanently delete "${account.value.name}" and ALL of its balance snapshots? This cannot be undone.`,
    { title: 'Delete Account?', kind: 'warning' },
  )
  if (!ok) return
  accountActionBusy.value = true
  try {
    await store.remove(account.value.id)
    router.push({ name: 'accounts' })
  } catch (err) {
    toast.add({ title: 'Failed to delete account', description: String(err), color: 'error' })
  } finally {
    accountActionBusy.value = false
  }
}
</script>

<template>
  <PageError v-if="error" :message="error" @retry="retry" class="m-6" />
  <div v-else-if="account" class="p-6">
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
        <UButton :disabled="accountActionBusy" @click="editAccountModalOpen = true">Edit</UButton>
        <UDropdownMenu :items="headerMenuItems">
          <UButton variant="ghost" color="neutral" icon="i-ph-dots-three" aria-label="More options" :loading="accountActionBusy" :disabled="accountActionBusy" />
        </UDropdownMenu>
      </div>
    </div>

    <!-- Current balance stat -->
    <div v-if="currentBalance !== null" class="relative mb-6">
      <StatCard
        label="Current Balance"
        :value="fmtMoney(displayBalance)"
        :hint="`as of ${currentBalanceAsOf}`"
      />
      <div
        v-if="milestoneCelebrating"
        :key="moteKey"
        class="pointer-events-none absolute inset-0 overflow-hidden"
        aria-hidden="true"
      >
        <span v-for="i in MOTE_COUNT" :key="i" class="tmfi-mote" :style="moteStyle(i)" />
      </div>
    </div>

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
      <div class="flex items-center gap-2">
        <UButton size="sm" variant="ghost" color="neutral" icon="i-ph-arrow-clockwise" :loading="rebuildingSnapshots" :disabled="rebuildingSnapshots" @click="rebuildSnapshots">Rebuild</UButton>
        <UButton size="sm" icon="i-ph-plus" @click="addModalOpen = true">Add Snapshot</UButton>
      </div>
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
            v-for="(row, i) in cachedRows(item.value)"
            :key="row.id"
            class="grid items-center py-1.5 not-first:border-t border-default tmfi-rise"
            :style="{ animationDelay: `${Math.min(i * 28, 196)}ms` }"
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
                <UButton size="xs" variant="ghost" color="error" icon="i-ph-trash" aria-label="Delete snapshot" :loading="deletingSnapshotId === row.id" :disabled="deletingSnapshotId !== null" @click="deleteSnapshot(row)" />
              </div>
            </template>
            <template v-else>
              <DateInput v-model="editDate" class="text-xs" />
              <CurrencyInput v-model="editBalance" class="text-sm mx-2" />
              <div class="flex items-center justify-end gap-1">
                <UButton size="xs" :loading="savingSnapshotId === row.id" :disabled="savingSnapshotId !== null" @click="saveEdit(row)">Save</UButton>
                <UButton size="xs" variant="ghost" color="neutral" :disabled="savingSnapshotId !== null" @click="cancelEdit">Cancel</UButton>
              </div>
            </template>
          </div>
        </template>
      </UAccordion>
    </div>

    <!-- Upkeep & improvements (real estate) -->
    <div v-if="isRealEstate" class="mb-8">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold">Upkeep &amp; Improvements</h2>
        <UButton size="sm" icon="i-ph-plus" @click="openAddAssetEvent">Add Event</UButton>
      </div>

      <div class="grid grid-cols-2 gap-3 mb-3">
        <div class="border border-default rounded-lg p-3">
          <p class="text-xs uppercase tracking-wide text-muted">Improvements (added to basis)</p>
          <p class="text-lg font-semibold font-mono mt-1 text-success">{{ fmtMoney(basisAdded) }}</p>
        </div>
        <div class="border border-default rounded-lg p-3">
          <p class="text-xs uppercase tracking-wide text-muted">Lifetime upkeep</p>
          <p class="text-lg font-semibold font-mono mt-1">{{ fmtMoney(lifetimeUpkeep) }}</p>
        </div>
      </div>

      <div v-if="assetEvents.length === 0" class="border border-default rounded-lg px-4 py-8 text-center text-sm text-muted">
        No upkeep or improvements logged yet.
      </div>
      <div v-else class="border border-default rounded-lg overflow-hidden">
        <table class="w-full text-sm">
          <tbody>
            <tr v-for="e in assetEvents" :key="e.id" class="not-first:border-t border-default">
              <td class="px-4 py-2 whitespace-nowrap text-muted">{{ fmtDate(e.date) }}</td>
              <td class="px-4 py-2">
                <UBadge :color="colorForAssetEventKind(e.kind)" variant="subtle" size="xs">{{ labelForAssetEventKind(e.kind) }}</UBadge>
              </td>
              <td class="px-4 py-2">
                <span>{{ e.description }}</span>
                <span v-if="e.vendor" class="text-muted"> · {{ e.vendor }}</span>
              </td>
              <td class="px-4 py-2 text-right font-mono">{{ fmtMoney(e.cost) }}</td>
              <td class="px-4 py-2 text-right whitespace-nowrap">
                <UButton size="xs" variant="ghost" color="neutral" icon="i-ph-pencil" aria-label="Edit event" @click="openEditAssetEvent(e)" />
                <UButton size="xs" variant="ghost" color="error" icon="i-ph-trash" aria-label="Delete event" :loading="deletingAssetEventId === e.id" :disabled="deletingAssetEventId !== null" @click="removeAssetEvent(e)" />
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Danger zone -->
    <div v-if="!account.isActive" class="border border-error/30 rounded-lg p-4">
      <h3 class="text-sm font-semibold text-error mb-1">Danger Zone</h3>
      <p class="text-xs text-muted mb-3">Permanently deletes this account and all of its balance snapshots. Cannot be undone.</p>
      <UButton color="error" variant="outline" size="sm" :loading="accountActionBusy" :disabled="accountActionBusy" @click="deleteAccount">Delete Account</UButton>
    </div>

    <!-- Edit account modal -->
    <UModal v-model:open="editAccountModalOpen" title="Edit Account" class="w-100">
      <template #body>
        <AccountForm :key="account.id" :account="account" @saved="onAccountSaved" />
      </template>
    </UModal>

    <!-- Add snapshot modal -->
    <UModal v-model:open="addModalOpen" title="Add Snapshot" class="w-84" @after:enter="focusFirstField">
      <template #body>
        <form ref="addForm" class="space-y-4" @submit.prevent="submitAddSnapshot(saveMode)">
          <UFormField label="Date">
            <DateInput v-model="newDate" class="w-full" />
          </UFormField>
          <UFormField label="Balance">
            <CurrencyInput v-model="newBalance" class="w-full" />
          </UFormField>
          <div class="flex justify-end pt-2 w-full">
            <UFieldGroup class="w-full">
              <UButton type="submit" :loading="addingSnapshot" :disabled="addingSnapshot" block>{{ saveModeLabel }}</UButton>
              <UDropdownMenu :items="saveMenuItems" :content="{ align: 'end' }">
                <UButton
                  type="button"
                  color="primary"
                  icon="i-ph-caret-down"
                  aria-label="More save options"
                  :disabled="addingSnapshot"
                />
              </UDropdownMenu>
            </UFieldGroup>
          </div>
        </form>
      </template>
    </UModal>

    <!-- Asset event modal -->
    <UModal v-model:open="assetEventModalOpen" :title="editingAssetEvent ? 'Edit asset event' : 'Add asset event'" class="sm:w-[560px] max-w-full">
      <template #body>
        <AssetEventForm :editing="editingAssetEvent" :preset-account-id="accountId" @saved="onAssetEventSaved" />
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