<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watchEffect } from 'vue'
import { DateTime } from 'luxon'
import { useRoute } from 'vue-router'
import { useToast } from '@nuxt/ui/composables'
import { useTransactionsStore } from '../stores/transactions'
import { useAccountsStore } from '../stores/accounts'
import { transactionTypeItems, categoryItems, labelForCategory, labelForSuppressKind } from '../lib/transactions/constants'
import { classifyFlow, cashFlowTotals, savingsRate, type FlowDirection } from '../lib/transactions/flow'
import { computeMedian, MONTHLY_REFERENCE_WINDOW, ANNUAL_REFERENCE_WINDOW, type PeriodStats } from '../lib/transactions/stats'
import { pctVsMedian, changeColor, trendIcon, proratedSuffix, type TrendField } from '../lib/transactions/trends'
import { useReveal } from '../composables/useReveal'
import { useMonthEndPaycheckAttribution } from '../composables/useMonthEndPaycheckAttribution'
import { attributeToFundedMonth, fundedMonthISO, isMonthEndPaycheckRow, MONTH_END_WINDOW_DAYS } from '../lib/transactions/attribution'
import * as api from '../lib/api/transactions'
import { listSimpleFinPending } from '../lib/api/simplefin'
import type { SimpleFinPendingTransaction } from '../lib/types/SimpleFinPendingTransaction'
import TransactionForm from '../components/TransactionForm.vue'
import ImportWizard from '../components/ImportWizard.vue'
import TransactionChart from '../components/TransactionChart.vue'
import TransactionMonthlyBreakdown from '../components/TransactionMonthlyBreakdown.vue'
import MonthPicker from '../components/MonthPicker.vue'
import type { Transaction } from '../lib/types/Transaction'
import { confirm } from '@tauri-apps/plugin-dialog'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'

const store = useTransactionsStore()
const accountsStore = useAccountsStore()
const toast = useToast()
const { error, run, retry } = usePageData()
const route = useRoute()

// ─── Figures tick into place ────────────────────────────────────────────────
// A deliberate check-in deserves to feel alive: when a scope's data lands the
// stat figures and savings rate count up from zero into their final values,
// so progress reads as something that happened, not a number that was always
// there. `revealKey` re-keys the savings-rate sheen so it replays per reveal.
const { progress: reveal, play: playReveal } = useReveal(600)
const revealKey = ref(0)
function runReveal() {
  revealKey.value++
  playReveal()
}

// ─── Modals ───────────────────────────────────────────────────────────────────

const isModalOpen = ref(false)
const editing = ref<Transaction | null>(null)

function openAdd() { editing.value = null; isModalOpen.value = true }
function openEdit(t: Transaction) { editing.value = t; isModalOpen.value = true }
async function onSaved(close = true) { if (close) isModalOpen.value = false; await applyFilters() }

const isImportOpen = ref(false)
async function onImportDone() { isImportOpen.value = false; await applyFilters() }

const removingId = ref<number | null>(null)

async function removeRow(t: Transaction) {
  const ok = await confirm(`Delete "${t.description}"?`, { title: 'Delete transaction' })
  if (!ok) return
  removingId.value = t.id
  try {
    await store.remove(t.id)
    await applyFilters()
  } catch (err) {
    toast.add({ title: 'Failed to delete transaction', description: String(err), color: 'error' })
  } finally {
    removingId.value = null
  }
}

// ─── Month navigation ──────────────────────────────────────────────────────────

const selectedDate = ref(DateTime.now().startOf('month'))
const monthLabel = computed(() => selectedDate.value.toFormat('MMMM yyyy'))
const yearLabel = computed(() => selectedDate.value.toFormat('yyyy'))
const monthStart = computed(() => selectedDate.value.toISODate()!)
const monthEnd = computed(() => selectedDate.value.endOf('month').toISODate()!)

function onMonthChange(dt: DateTime) {
  selectedDate.value = dt
  applyFilters()
}

// ─── Date scope ────────────────────────────────────────────────────────────────

type Scope = 'month' | 'year' | 'all'
const scope = ref<Scope>('month')

function setScope(s: Scope) {
  scope.value = s
  applyFilters()
}

// ─── Chart mode ────────────────────────────────────────────────────────────────

const chartMode = ref<'breakdown' | 'cumulative'>('breakdown')

const chartTitle = computed(() => {
  if (chartMode.value === 'breakdown') {
    const label = scope.value === 'month' ? monthLabel.value
      : scope.value === 'year' ? yearLabel.value
      : 'All Time'
    return `Expense Breakdown — ${label}`
  }
  return scope.value === 'all' ? 'Cumulative Net — All Time' : `Cumulative Net — ${yearLabel.value}`
})

// ─── Secondary filters ─────────────────────────────────────────────────────────

const accountIds = ref<number[]>([])
const types = ref<string[]>([])
const categories = ref<string[]>([])
const searchTerms = ref<string[]>([])

// Rule-suppressed noise (fees/investment activity inside synced accounts) is
// hidden by default; this toggle reveals it in the table. Charts and totals
// always exclude it — classifyFlow neutralizes suppressed rows regardless.
const showSuppressed = ref(false)

async function toggleSuppressed() {
  showSuppressed.value = !showSuppressed.value
  await applyFilters()
}

async function clearFilters() {
  accountIds.value = []
  types.value = []
  categories.value = []
  searchTerms.value = []
  await applyFilters()
}

// ─── Data loading ──────────────────────────────────────────────────────────────

const yearTransactions = ref<Transaction[]>([])
// Full transaction set only loaded for the 'all time' scope (needed by the cumulative
// and breakdown charts). Median comparison uses periodStats instead — much lighter.
const allTimeTransactions = ref<Transaction[]>([])

// Per-period stats used for median comparison — fetched directly from the DB as
// lightweight aggregates rather than pulling all transaction rows to the frontend.
const monthlyPeriodStats = ref<PeriodStats[]>([])
const annualPeriodStats = ref<PeriodStats[]>([])

async function applyFilters() {
  let startDate: string | null = null
  let endDate: string | null = null

  if (scope.value === 'month') {
    startDate = monthStart.value
    endDate = monthEnd.value
  } else if (scope.value === 'year') {
    startDate = selectedDate.value.startOf('year').toISODate()!
    endDate = selectedDate.value.endOf('year').toISODate()!
  }

  await store.setFilter({
    accountIds: accountIds.value,
    types: types.value,
    categories: categories.value,
    searchTerms: searchTerms.value,
    includeSuppressed: showSuppressed.value,
    startDate,
    endDate,
  })

  // Always load year data — needed for annual stats card, cumulative chart
  // (month scope), and monthly breakdown when scope !== 'month'.
  await loadYearData()

  if (scope.value === 'all') {
    await loadAllTimeData()
  } else {
    allTimeTransactions.value = []
  }

  await loadPeriodStats()

  runReveal() // figures tick up once the scope's data has settled
}

async function loadYearData() {
  // The fetch reaches back into the prior December's month-end window so a
  // shifted year-end paycheck can be attributed into January. When the
  // attribution preference is off, attributedYear's date filter drops the
  // extra rows again.
  const result = await api.listTransactions({
    accountIds: accountIds.value,
    types: types.value,
    categories: categories.value,
    searchTerms: searchTerms.value,
    startDate: selectedDate.value.startOf('year').minus({ days: MONTH_END_WINDOW_DAYS }).toISODate()!,
    endDate: selectedDate.value.endOf('year').toISODate()!,
    limit: null,
  })
  yearTransactions.value = result.rows
}

async function loadAllTimeData() {
  const result = await api.listTransactions({
    accountIds: accountIds.value,
    types: types.value,
    categories: categories.value,
    searchTerms: searchTerms.value,
    limit: null,
  })
  allTimeTransactions.value = result.rows
}

async function loadPeriodStats() {
  const secondary = {
    accountIds: accountIds.value,
    types: types.value,
    categories: categories.value,
    searchTerms: searchTerms.value,
  }
  // The in-progress calendar period never enters the baseline (partial data
  // skews the medians), and when the *selected* period is itself in progress,
  // reference periods are prorated to today's point-in-period so a 3-day-old
  // month isn't compared against full-month typicals.
  const now = DateTime.now()
  const [monthly, annual] = await Promise.all([
    api.periodStats({
      ...secondary,
      groupBy: 'month',
      excludePeriod: selectedDate.value.toFormat('yyyy-MM'),
      currentPeriod: now.toFormat('yyyy-MM'),
      throughDate: selectedDate.value.hasSame(now, 'month') ? now.toISODate() : null,
      attributePaycheckToNextMonth: attributePaychecks.value,
    }),
    api.periodStats({
      ...secondary,
      groupBy: 'year',
      excludePeriod: selectedDate.value.toFormat('yyyy'),
      currentPeriod: now.toFormat('yyyy'),
      throughDate: selectedDate.value.hasSame(now, 'year') ? now.toISODate() : null,
      attributePaycheckToNextMonth: attributePaychecks.value,
    }),
  ])
  monthlyPeriodStats.value = monthly
  annualPeriodStats.value = annual
}

// ─── Infinite scroll ───────────────────────────────────────────────────────────

const sentinel = ref<HTMLElement | null>(null)

const observer = new IntersectionObserver(
  (entries) => { if (entries[0].isIntersecting) store.loadMore() },
  { rootMargin: '200px' }
)

watchEffect(() => {
  observer.disconnect()
  if (sentinel.value) observer.observe(sentinel.value)
})

onUnmounted(() => observer.disconnect())

// ─── Totals ────────────────────────────────────────────────────────────────────

// Analytics run on attribution-adjusted data: with the paycheck-funded-month
// preference on, month-end paycheck rows are re-dated to the month they fund.
// The date filter also drops the prior-December window rows loadYearData
// over-fetches (and, when shifting, this year's shifted-out December tail).
const { enabled: attributePaychecks } = useMonthEndPaycheckAttribution()

const attributedYear = computed(() => {
  const start = selectedDate.value.startOf('year').toISODate()!
  const end = selectedDate.value.endOf('year').toISODate()!
  return attributeToFundedMonth(yearTransactions.value, attributePaychecks.value)
    .filter((t) => t.date >= start && t.date <= end)
})

// Monthly analytics always derive from the year data rather than the table's
// store page — the page is paginated, and attribution needs the neighbouring
// months' rows anyway.
const monthlyTransactions = computed(() => {
  const start = monthStart.value
  const end = monthEnd.value
  return attributedYear.value.filter((t) => t.date >= start && t.date <= end)
})

const monthlyTotals = computed(() => cashFlowTotals(monthlyTransactions.value, accountsStore.accounts))
const annualTotals = computed(() => cashFlowTotals(attributedYear.value, accountsStore.accounts))

// The savings rate is the number a FIRE check-in is really about. Crossing 50%
// — the canonical "halfway to your time" rate — is a genuine milestone, so a
// strong month earns the emerald voice of progress and a single sheen across
// the figure. Below that it stays the calm informational blue.
const STRONG_RATE = 0.5

const monthlyRate = computed(() => savingsRate(monthlyTotals.value))
const annualRate = computed(() => savingsRate(annualTotals.value))

function fmtRate(rate: number | null): string {
  return rate == null ? '—' : (rate * 100 * reveal.value).toFixed(1) + '%'
}
function rateColor(rate: number | null, savings: number): string {
  if (rate != null && rate >= STRONG_RATE) return 'text-primary'
  return savings > 0 ? 'text-info' : 'text-muted'
}
function isStrongRate(rate: number | null): boolean {
  return rate != null && rate >= STRONG_RATE
}

// ─── Median comparisons ───────────────────────────────────────────────────────

// Period stats are pre-aggregated by the Rust command (grouped, classified,
// selected + in-progress periods excluded) — computeMedian takes the median
// across the most recent rows, so "typical" reflects recent behaviour rather
// than years-old history that may predate tracking some flows.
const medianMonthly = computed(() => computeMedian(monthlyPeriodStats.value, MONTHLY_REFERENCE_WINDOW))
const medianAnnual = computed(() => computeMedian(annualPeriodStats.value, ANNUAL_REFERENCE_WINDOW))

const activeTotals = computed(() => {
  if (scope.value === 'month') return monthlyTotals.value
  if (scope.value === 'year') return annualTotals.value
  return null
})

const activeRate = computed(() => {
  if (scope.value === 'month') return monthlyRate.value
  if (scope.value === 'year') return annualRate.value
  return null
})

const activeTransactionCount = computed(() => {
  if (scope.value === 'month') return monthlyTransactions.value.length
  if (scope.value === 'year') return attributedYear.value.length
  return 0
})

const activeMedian = computed(() => {
  if (scope.value === 'month') return medianMonthly.value
  if (scope.value === 'year') return medianAnnual.value
  return null
})

// Only show comparison when there are at least 2 reference periods so the
// baseline is meaningful (not just the one other month the user has entered).
const showComparison = computed(() => (activeMedian.value?.periodCount ?? 0) >= 2)

// A field whose median is exactly 0 has no meaningful baseline — usually every
// reference period predates tracking that flow — so its "typ." line is hidden
// rather than showing a bogus $0.00.
function baselineFor(field: TrendField): number | null {
  const v = activeMedian.value?.totals[field] ?? 0
  return v === 0 ? null : v
}

// When the selected period is in progress the baseline is prorated to today's
// point-in-period — say so, or "typ." reads as a full-period figure.
const typSuffix = computed(() =>
  scope.value === 'all' ? '' : proratedSuffix(scope.value, selectedDate.value)
)

function medianRate(totals: { income: number; savings: number }): string {
  if (totals.income <= 0) return '—'
  return (totals.savings / totals.income * 100).toFixed(1) + '%'
}

// Median savings rate as a fraction, for the trend arrow/colour on the hero.
const medianRateValue = computed(() => {
  const m = activeMedian.value?.totals
  if (!m || m.income <= 0) return null
  return m.savings / m.income
})

const rateTrendPct = computed(() => {
  if (activeRate.value == null || medianRateValue.value == null) return null
  return pctVsMedian(activeRate.value, medianRateValue.value)
})

// ─── Chart data (scope-aware, never paginated) ────────────────────────────────

// All-time analytics get the same attribution treatment; no boundary trimming
// needed since every shifted row still lands inside "all time".
const attributedAllTime = computed(() =>
  attributeToFundedMonth(allTimeTransactions.value, attributePaychecks.value)
)

// Expense breakdown uses scope-appropriate full data set.
const breakdownTransactions = computed(() => {
  if (scope.value === 'month') return monthlyTransactions.value
  if (scope.value === 'year') return attributedYear.value
  return attributedAllTime.value
})

// Cumulative chart always uses at least the full year; all-time when scope='all'.
const cumulativeTransactions = computed(() =>
  scope.value === 'all' ? attributedAllTime.value : attributedYear.value
)

// ─── Carried-in month-end paychecks ───────────────────────────────────────────

// Rows dated before the selected period that the attribution preference counts
// toward it (last month's month-end paycheck and its pre-tax contributions).
// They're inside every total above but outside the store's date filter, so
// they join the ledger table as badged rows — otherwise the figures don't add
// up to what the table shows.
const carriedInTransactions = computed(() => {
  if (!attributePaychecks.value || scope.value === 'all') return []
  const start = scope.value === 'month' ? monthStart.value : selectedDate.value.startOf('year').toISODate()!
  const end = scope.value === 'month' ? monthEnd.value : selectedDate.value.endOf('year').toISODate()!
  return yearTransactions.value.filter((t) => {
    if (t.date >= start) return false
    const funded = fundedMonthISO(t)
    return funded != null && funded >= start && funded <= end
  })
})

const carriedFromLabel = computed(() => {
  const prev = scope.value === 'month'
    ? selectedDate.value.minus({ months: 1 })
    : selectedDate.value.startOf('year').minus({ months: 1 })
  return prev.toFormat(scope.value === 'month' ? 'MMMM' : 'MMMM yyyy')
})

// A month-end paycheck row in the ledger whose cash flow was shifted out of
// its calendar month; marked so its exclusion from the totals is explainable.
function isShiftedOut(t: Transaction): boolean {
  return attributePaychecks.value && isMonthEndPaycheckRow(t)
}

function shiftedOutTitle(t: Transaction): string {
  const funded = fundedMonthISO(t)
  const label = funded ? DateTime.fromISO(funded).toFormat('MMMM') : 'next month'
  return `Month-end paycheck — counted toward ${label}`
}

// ─── Pending (SimpleFIN) ──────────────────────────────────────────────────────

// Transactions still pending at the bank, shown for awareness in the ledger
// table. They live outside `txn` entirely, so no sum, chart, or stat on this
// page (or anywhere else) can see them — every pending cell is muted to signal
// "not counted yet". Deliberately unaffected by the secondary filters: the set
// is small and vanishes as rows post.
const pendingTransactions = ref<SimpleFinPendingTransaction[]>([])

// Only pending rows dated inside the viewed period join the table — today's
// pending charge doesn't belong in last March's ledger.
const scopedPending = computed(() => {
  if (scope.value === 'all') return pendingTransactions.value
  const start = scope.value === 'month' ? monthStart.value : selectedDate.value.startOf('year').toISODate()!
  const end = scope.value === 'month' ? monthEnd.value : selectedDate.value.endOf('year').toISODate()!
  return pendingTransactions.value.filter((p) => p.date >= start && p.date <= end)
})

// What the pending set would add to the period's net if everything posts as-is
// (same sign convention as the verdict strip: net = income − expenses).
const pendingNet = computed(() => {
  let net = 0
  for (const p of scopedPending.value) net += p.txnType === 'income' ? p.amount : -p.amount
  return net
})

// ─── Table ─────────────────────────────────────────────────────────────────────

// The ledger interleaves three kinds of rows: bank-pending (awareness only,
// muted, outside every total), the posted page from the store, and rows
// carried in from the prior month-end (inside the totals but dated before the
// period, so outside the store's date filter). Pending rows are the newest so
// they lead; carried rows are the period's oldest by real date so they trail —
// infinite scroll appends posted pages in between.
type LedgerRow =
  | { kind: 'posted' | 'carried'; txn: Transaction }
  | { kind: 'pending'; txn: SimpleFinPendingTransaction }

const rows = computed<LedgerRow[]>(() => [
  ...scopedPending.value.map((txn) => ({ kind: 'pending' as const, txn })),
  ...store.page.rows.map((txn) => ({ kind: 'posted' as const, txn })),
  ...carriedInTransactions.value.map((txn) => ({ kind: 'carried' as const, txn })),
])

function rowMuted(r: LedgerRow): boolean {
  return r.kind === 'pending'
}

function rowAccountLabel(r: LedgerRow): string {
  const base = accountName(r.txn.accountId)
  if (r.kind !== 'pending' && r.txn.type === 'transfer') {
    return `${base} → ${accountName(r.txn.transferAccountId)}`
  }
  return base
}

function rowDateIcon(r: LedgerRow): { name: string; title: string } | null {
  if (r.kind === 'carried') {
    return {
      name: 'i-ph-arrow-bend-down-right',
      title: `Carried in from ${carriedFromLabel.value} — month-end paycheck counted toward ${tableScopeLabel.value}'s totals`,
    }
  }
  if (r.kind === 'posted' && isShiftedOut(r.txn)) {
    return { name: 'i-ph-arrow-bend-up-right', title: shiftedOutTitle(r.txn) }
  }
  return null
}

type RowBadge = { label: string; color: 'neutral' | 'info' | 'warning'; icon: string; title?: string }

// Status badge in the category column. Pending/carried notation wins over the
// suppressed/contribution badges — the row's relationship to the totals is the
// thing that needs explaining; the date-cell icon still carries the detail.
function rowBadge(r: LedgerRow): RowBadge | null {
  if (r.kind === 'pending') {
    return {
      label: 'Pending',
      color: 'warning',
      icon: 'i-ph-clock',
      title: 'Pending at your bank — not counted in any totals until it posts',
    }
  }
  if (r.kind === 'carried') {
    return {
      label: `From ${carriedFromLabel.value}`,
      color: 'info',
      icon: 'i-ph-arrow-bend-down-right',
      title: `Month-end paycheck counted toward ${tableScopeLabel.value}'s totals`,
    }
  }
  if (r.txn.suppressedAs) {
    return { label: labelForSuppressKind(r.txn.suppressedAs), color: 'neutral', icon: 'i-ph-eye-slash' }
  }
  if (r.txn.isContribution) {
    return r.txn.isWithdrawal
      ? { label: 'Withdrawal', color: 'warning', icon: 'i-ph-arrow-line-up' }
      : { label: 'Contribution', color: 'info', icon: 'i-ph-piggy-bank' }
  }
  return null
}

function rowCategory(r: LedgerRow): { label: string; muted: boolean } {
  return r.kind === 'pending' ? { label: 'Pending', muted: true } : categoryCell(r.txn)
}

function rowFlowIcon(r: LedgerRow): string {
  if (r.kind === 'pending') return r.txn.txnType === 'income' ? 'i-ph-arrow-right' : 'i-ph-arrow-left'
  return flowIcon(r.txn)
}

function rowFlowColor(r: LedgerRow): string {
  return r.kind === 'pending' ? 'text-muted' : flowColor(r.txn)
}

function rowDirectionLabel(r: LedgerRow): string {
  if (r.kind === 'pending') return r.txn.txnType === 'income' ? 'Income (pending)' : 'Expense (pending)'
  return directionLabel(r.txn)
}

function rowId(r: LedgerRow): number | null {
  return r.kind === 'pending' ? null : r.txn.id
}

function editRow(r: LedgerRow) {
  if (r.kind !== 'pending') openEdit(r.txn)
}

function deleteRow(r: LedgerRow) {
  if (r.kind !== 'pending') removeRow(r.txn)
}

const tableScopeLabel = computed(() => {
  if (scope.value === 'month') return monthLabel.value
  if (scope.value === 'year') return yearLabel.value
  return 'All time'
})

const columns = [
  { id: 'date', header: 'Date' },
  { id: 'description', header: 'Description' },
  { id: 'account', header: 'Account' },
  { id: 'category', header: 'Category' },
  { id: 'amount', header: 'Amount', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'actions', header: '', meta: { class: { td: 'text-right' } } },
]

function accountName(id: number | null): string {
  if (id == null) return '—'
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

// ─── Amount direction indicator ──────────────────────────────────────────────

const DIRECTION_COLOR: Record<FlowDirection, string> = {
  inflow: 'text-success',
  outflow: 'text-error',
  neutral: 'text-muted',
}

function flowIcon(t: Transaction): string {
  const { direction, isTransfer } = classifyFlow(t, accountsStore.accounts)
  if (isTransfer) return 'i-ph-arrows-left-right'
  return direction === 'inflow' ? 'i-ph-arrow-right' : 'i-ph-arrow-left'
}

function flowColor(t: Transaction): string {
  return DIRECTION_COLOR[classifyFlow(t, accountsStore.accounts).direction]
}

function directionLabel(t: Transaction): string {
  const { direction, isTransfer } = classifyFlow(t, accountsStore.accounts)
  if (!isTransfer) return direction === 'inflow' ? 'Income' : 'Expense'
  if (direction === 'inflow') return 'Transfer in'
  if (direction === 'outflow') return 'Transfer out'
  return 'Transfer'
}

// ─── Category cell ───────────────────────────────────────────────────────────

// The stored category only means something on expenses. Income and transfers
// show what they ARE instead of "Uncategorized" — except a transfer into a
// count-payments-as-expense account (mortgage, car loan), where the payment IS
// the expense: show the bucket it actually lands in.
function categoryCell(t: Transaction): { label: string; muted: boolean } {
  if (t.type === 'income') return { label: 'Income', muted: true }
  if (t.type === 'transfer') {
    const dest = accountsStore.accounts.find((a) => a.id === t.transferAccountId)
    if (dest?.countPaymentsAsExpense) {
      return { label: labelForCategory(t.category === 'discretionary' ? 'discretionary' : 'fixed'), muted: false }
    }
    return { label: 'Transfer', muted: true }
  }
  return { label: labelForCategory(t.category), muted: false }
}

// Arriving from Expenses' merchant/category drill-down seeds the filters and
// scope, so the table shows exactly what fed the figure that was clicked —
// same expense-type filter, same time period, plus the search/category term.
// Note: this deliberately does NOT clear the query afterward — App.vue keys the
// routed component on `route.fullPath` (to force a remount after background
// sync catch-up), so a post-mount `router.replace` here would trigger a second
// remount and wipe the filters state we just set.
function seedFiltersFromRoute() {
  const { search, category, types: typesParam, period, date } = route.query

  if (typeof search === 'string') searchTerms.value = [search]
  if (typeof category === 'string') categories.value = [category]
  if (typeof typesParam === 'string') types.value = [typesParam]
  if (period === 'month' || period === 'year' || period === 'all') scope.value = period
  if (typeof date === 'string') {
    const dt = DateTime.fromISO(date)
    if (dt.isValid) selectedDate.value = dt.startOf('month')
  }
}

onMounted(() => run(async () => {
  await accountsStore.load()
  seedFiltersFromRoute()
  await applyFilters()
  pendingTransactions.value = await listSimpleFinPending()
}))
</script>

<template>
  <div class="p-6 space-y-4">
    <PageError v-if="error" :message="error" @retry="retry" />

    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Transactions</h1>
      <div class="flex gap-2">
        <UButton variant="subtle" icon="i-ph-upload" @click="isImportOpen = true">Import CSV</UButton>
        <UButton icon="i-ph-plus" @click="openAdd">Add transaction</UButton>
      </div>
    </div>

    <!-- Chart card -->
    <div class="border border-default rounded-lg p-4">
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-3">
          <MonthPicker
            v-if="scope !== 'all'"
            :model-value="selectedDate"
            :mode="scope === 'year' ? 'year' : 'month'"
            @update:model-value="onMonthChange"
          />
          <div class="flex gap-1">
            <UButton
              size="xs"
              :variant="scope === 'month' ? 'subtle' : 'ghost'"
              :color="scope === 'month' ? 'primary' : 'neutral'"
              @click="setScope('month')"
            >Month</UButton>
            <UButton
              size="xs"
              :variant="scope === 'year' ? 'subtle' : 'ghost'"
              :color="scope === 'year' ? 'primary' : 'neutral'"
              @click="setScope('year')"
            >Year</UButton>
            <UButton
              size="xs"
              :variant="scope === 'all' ? 'subtle' : 'ghost'"
              :color="scope === 'all' ? 'primary' : 'neutral'"
              @click="setScope('all')"
            >All time</UButton>
          </div>
        </div>
        <div class="flex gap-1">
          <UButton
            size="xs"
            :variant="chartMode === 'breakdown' ? 'subtle' : 'ghost'"
            :color="chartMode === 'breakdown' ? 'primary' : 'neutral'"
            @click="chartMode = 'breakdown'"
          >Expense Breakdown</UButton>
          <UButton
            size="xs"
            :variant="chartMode === 'cumulative' ? 'subtle' : 'ghost'"
            :color="chartMode === 'cumulative' ? 'primary' : 'neutral'"
            @click="chartMode = 'cumulative'"
          >Cumulative Chart</UButton>
        </div>
      </div>
      <!-- Verdict strip — the period's headline figures, always visible across
           both chart modes (the savings rate must survive the toggle). Hidden
           for all-time scope, which has no single-period total. -->
      <div v-if="scope !== 'all' && activeTotals" class="flex flex-wrap items-start gap-x-10 gap-y-4 pb-4 mb-4 border-b border-default">
        <!-- Savings rate — the number a FIRE check-in is really about -->
        <div class="shrink-0">
          <div class="relative inline-block overflow-hidden">
            <p class="text-3xl font-bold tabular-nums leading-none" :class="rateColor(activeRate, activeTotals.savings)">{{ fmtRate(activeRate) }}</p>
            <span v-if="isStrongRate(activeRate)" :key="`r-${revealKey}`" class="tmfi-sheen" />
          </div>
          <p class="text-xs text-muted mt-1.5">Savings rate</p>
          <p v-if="showComparison && medianRateValue != null" class="text-xs flex items-center gap-1 mt-0.5">
            <UIcon :name="trendIcon(rateTrendPct)" class="size-3 shrink-0" :class="changeColor('savings', rateTrendPct)" />
            <span class="text-dimmed tabular-nums">typ. {{ medianRate(activeMedian!.totals) }}{{ typSuffix }}</span>
          </p>
        </div>

        <div class="hidden lg:block w-px self-stretch bg-default shrink-0" />

        <!-- Income / Expenses / Net — content-sized so values never collide -->
        <div class="flex flex-wrap items-start gap-x-10 gap-y-4 flex-1 min-w-0">
          <div class="shrink-0">
            <p class="text-lg font-semibold tabular-nums leading-none text-success">{{ money(activeTotals.income * reveal) }}</p>
            <p class="text-xs text-muted mt-1.5">Income</p>
            <p v-if="showComparison && baselineFor('income') != null" class="text-xs flex items-center gap-1 mt-0.5">
              <UIcon :name="trendIcon(pctVsMedian(activeTotals.income, baselineFor('income')!))" class="size-3 shrink-0" :class="changeColor('income', pctVsMedian(activeTotals.income, baselineFor('income')!))" />
              <span class="text-dimmed tabular-nums">typ. {{ money(baselineFor('income')!) }}{{ typSuffix }}</span>
            </p>
          </div>
          <div class="shrink-0">
            <p class="text-lg font-semibold tabular-nums leading-none text-error">{{ money(activeTotals.expense * reveal) }}</p>
            <p class="text-xs text-muted mt-1.5">Expenses</p>
            <p v-if="showComparison && baselineFor('expense') != null" class="text-xs flex items-center gap-1 mt-0.5">
              <UIcon :name="trendIcon(pctVsMedian(activeTotals.expense, baselineFor('expense')!))" class="size-3 shrink-0" :class="changeColor('expense', pctVsMedian(activeTotals.expense, baselineFor('expense')!))" />
              <span class="text-dimmed tabular-nums">typ. {{ money(baselineFor('expense')!) }}{{ typSuffix }}</span>
            </p>
          </div>
          <div class="shrink-0">
            <p class="text-lg font-semibold tabular-nums leading-none" :class="activeTotals.net >= 0 ? 'text-heading' : 'text-error'">{{ money(activeTotals.net * reveal) }}</p>
            <p class="text-xs text-muted mt-1.5">Net</p>
            <p v-if="showComparison && baselineFor('net') != null" class="text-xs flex items-center gap-1 mt-0.5">
              <UIcon :name="trendIcon(pctVsMedian(activeTotals.net, baselineFor('net')!))" class="size-3 shrink-0" :class="changeColor('net', pctVsMedian(activeTotals.net, baselineFor('net')!))" />
              <span class="text-dimmed tabular-nums">typ. {{ money(baselineFor('net')!) }}{{ typSuffix }}</span>
            </p>
          </div>
        </div>

        <span class="text-xs text-muted shrink-0">{{ activeTransactionCount }} transactions</span>
      </div>

      <p class="text-sm text-muted mb-3">{{ chartTitle }}</p>
      <template v-if="chartMode === 'breakdown'">
        <TransactionMonthlyBreakdown
          v-if="breakdownTransactions.length > 0"
          :transactions="breakdownTransactions"
          :accounts="accountsStore.accounts"
        />
        <div v-else class="flex flex-col items-center gap-2 py-10 text-center">
          <UIcon name="i-ph-chart-line-up" class="size-7 text-dimmed" />
          <p class="text-sm text-muted">Nothing to chart for this period yet.</p>
        </div>
      </template>
      <template v-else>
        <TransactionChart
          v-if="cumulativeTransactions.length > 0"
          :transactions="cumulativeTransactions"
          :accounts="accountsStore.accounts"
        />
        <div v-else class="flex flex-col items-center gap-2 py-10 text-center">
          <UIcon name="i-ph-chart-line-up" class="size-7 text-dimmed" />
          <p class="text-sm text-muted">Nothing to chart for this period yet.</p>
        </div>
      </template>
    </div>

    <!-- Secondary filters -->
    <div class="flex flex-wrap gap-2 items-end">
      <USelectMenu
        v-model="accountIds"
        :items="accountsStore.accounts.map((a) => ({ label: a.name, value: a.id }))"
        value-key="value"
        multiple
        placeholder="All accounts"
        class="w-44"
      />
      <USelectMenu
        v-model="types"
        :items="transactionTypeItems"
        value-key="value"
        multiple
        placeholder="All types"
        class="w-36"
      />
      <USelectMenu
        v-model="categories"
        :items="categoryItems"
        value-key="value"
        multiple
        placeholder="All categories"
        class="w-40"
      />
      <UInputTags v-model="searchTerms" placeholder="Search terms" class="flex-1 min-w-0" />
      <UButton @click="applyFilters">Apply</UButton>
      <UButton variant="ghost" @click="clearFilters">Clear</UButton>
    </div>

    <!-- Table -->
    <!-- One ledger: pending rows lead (muted, badged, outside every total),
         posted rows follow, carried-in month-end rows trail (badged, inside
         the totals despite their earlier date). -->
    <div class="border border-default rounded-lg overflow-hidden">
      <div class="flex items-center justify-between px-4 py-2 border-b border-default">
        <p class="text-xs text-muted">{{ tableScopeLabel }}</p>
        <div class="flex items-center gap-3">
          <button
            v-if="store.page.suppressedCount > 0 || showSuppressed"
            class="text-xs flex items-center gap-1 text-muted hover:text-default"
            @click="toggleSuppressed"
          >
            <UIcon :name="showSuppressed ? 'i-ph-eye' : 'i-ph-eye-slash'" class="size-3.5" />
            {{ showSuppressed ? 'Hide' : 'Show' }} suppressed ({{ store.page.suppressedCount }})
          </button>
          <p class="text-xs text-muted">
            {{ store.page.rows.length }}{{ store.hasMore ? '+' : '' }} of {{ store.page.totalCount }} transactions<!--
            --><span v-if="scopedPending.length > 0" :title="`Net ${money(pendingNet)} if everything posts as-is`"> · {{ scopedPending.length }} pending — not counted until posted</span><!--
            --><span v-if="carriedInTransactions.length > 0"> · {{ carriedInTransactions.length }} carried in from {{ carriedFromLabel }}</span>
          </p>
        </div>
      </div>
      <UTable :data="rows" :columns="columns">
        <template #empty>
          <div class="flex flex-col items-center gap-3 py-12 text-center">
            <UIcon name="i-ph-receipt" class="size-8 text-dimmed" />
            <div>
              <p class="text-sm font-medium text-heading">
                {{ scope === 'all' ? 'No transactions yet' : `Nothing recorded for ${tableScopeLabel}` }}
              </p>
              <p class="text-xs text-muted mt-1">Add one by hand or import a CSV — your numbers start here.</p>
            </div>
            <div class="flex gap-2">
              <UButton size="xs" variant="subtle" icon="i-ph-upload" @click="isImportOpen = true">Import CSV</UButton>
              <UButton size="xs" icon="i-ph-plus" @click="openAdd">Add transaction</UButton>
            </div>
          </div>
        </template>
        <template #date-cell="{ row }">
          <span class="inline-flex items-center gap-1.5" :class="{ 'text-muted': rowMuted(row.original) }">
            {{ row.original.txn.date }}
            <UIcon
              v-if="rowDateIcon(row.original)"
              :name="rowDateIcon(row.original)!.name"
              class="size-3.5 text-info shrink-0"
              :title="rowDateIcon(row.original)!.title"
            />
          </span>
        </template>
        <template #description-cell="{ row }">
          <!-- Tooltip shows the bank's unedited text when the import cleaned it up. -->
          <span class="block max-w-[300px] truncate" :class="{ 'text-muted': rowMuted(row.original) }" :title="row.original.txn.rawDescription ?? row.original.txn.description">{{ row.original.txn.description }}</span>
        </template>
        <template #account-cell="{ row }">
          <span :class="{ 'text-muted': rowMuted(row.original) }">{{ rowAccountLabel(row.original) }}</span>
        </template>
        <template #category-cell="{ row }">
          <div class="flex items-center gap-1.5">
            <UBadge
              v-if="rowBadge(row.original)"
              :color="rowBadge(row.original)!.color"
              variant="subtle"
              size="sm"
              :icon="rowBadge(row.original)!.icon"
              :title="rowBadge(row.original)!.title"
            >{{ rowBadge(row.original)!.label }}</UBadge>
            <span v-else :class="{ 'text-muted': rowCategory(row.original).muted }">{{ rowCategory(row.original).label }}</span>
          </div>
        </template>
        <template #amount-cell="{ row }">
          <span class="inline-flex items-center justify-end gap-1.5" :class="rowFlowColor(row.original)">
            <UIcon :name="rowFlowIcon(row.original)" class="size-4 shrink-0" :title="rowDirectionLabel(row.original)" />
            {{ money(row.original.txn.amount) }}
          </span>
        </template>
        <template #actions-cell="{ row }">
          <template v-if="row.original.kind !== 'pending'">
            <UButton size="xs" variant="ghost" icon="i-ph-pencil" @click="editRow(row.original)" />
            <UButton size="xs" variant="ghost" color="error" icon="i-ph-trash" :loading="removingId === rowId(row.original)" :disabled="removingId !== null" @click="deleteRow(row.original)" />
          </template>
        </template>
      </UTable>
      <div ref="sentinel" class="h-1" />
      <div v-if="store.loading && store.page.rows.length > 0" class="flex justify-center py-3">
        <UIcon name="i-ph-spinner" class="size-5 text-muted animate-spin" />
      </div>
    </div>

    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit transaction' : 'Add transaction'">
      <template #body>
        <TransactionForm :editing="editing" @saved="onSaved" @cancel="isModalOpen = false" />
      </template>
    </UModal>

    <UModal v-model:open="isImportOpen" title="Import transactions from CSV" class="max-w-full w-4/5 lg:w-[1000px]">
      <template #body>
        <ImportWizard @done="onImportDone" />
      </template>
    </UModal>
  </div>
</template>
