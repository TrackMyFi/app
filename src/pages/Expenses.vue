<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { useRouter } from 'vue-router'
import { useAccountsStore } from '../stores/accounts'
import { usePageData } from '../composables/usePageData'
import { useReveal } from '../composables/useReveal'
import * as api from '../lib/api/transactions'
import * as vendorRulesApi from '../lib/api/vendorRules'
import { classifyFlow } from '../lib/transactions/flow'
import { computeMedian, type PeriodStats } from '../lib/transactions/stats'
import { pctVsMedian, changeColor, trendIcon } from '../lib/transactions/trends'
import { CATEGORY_LABELS, CATEGORY_TEXT_COLOR } from '../lib/transactions/constants'
import { groupByMerchant, type VendorRuleInput } from '../lib/expenses/merchants'
import { detectRecurring } from '../lib/expenses/recurring'
import { detectCategorySpikes, type OpportunityItem } from '../lib/expenses/opportunities'
import MonthPicker from '../components/MonthPicker.vue'
import ExpenseMerchantList from '../components/ExpenseMerchantList.vue'
import ExpenseOpportunities from '../components/ExpenseOpportunities.vue'
import PageError from '../components/PageError.vue'
import type { Transaction } from '../lib/types/Transaction'

const router = useRouter()
const accountsStore = useAccountsStore()
const { error, run, retry } = usePageData()
const { progress: reveal, play: playReveal } = useReveal(600)

// ─── Scope (mirrors Transactions.vue's Month/Year/All-time toggle) ─────────────

type Scope = 'month' | 'year' | 'all'
const scope = ref<Scope>('month')
const selectedDate = ref(DateTime.now().startOf('month'))

const monthLabel = computed(() => selectedDate.value.toFormat('MMMM yyyy'))
const yearLabel = computed(() => selectedDate.value.toFormat('yyyy'))
const periodLabel = computed(() => {
  if (scope.value === 'all') return 'All time'
  return scope.value === 'year' ? yearLabel.value : monthLabel.value
})
const periodWord = computed<'month' | 'year'>(() => (scope.value === 'year' ? 'year' : 'month'))

function onMonthChange(dt: DateTime) {
  selectedDate.value = dt
  applyFilters()
}

function setScope(s: Scope) {
  scope.value = s
  applyFilters()
}

// ─── Data ───────────────────────────────────────────────────────────────────────

const scopeTransactions = ref<Transaction[]>([])
const monthlyPeriodStats = ref<PeriodStats[]>([])
const annualPeriodStats = ref<PeriodStats[]>([])
// Recurring-charge detection always looks at a fixed trailing window, independent
// of the scope toggle — a subscription is still active whether or not you're
// currently viewing "this month".
const recurringWindowTransactions = ref<Transaction[]>([])
const hasAnyTransactions = ref(true)
const vendorRules = ref<VendorRuleInput[]>([])

async function loadVendorRules() {
  const rules = await vendorRulesApi.listVendorRules()
  vendorRules.value = rules.map((r) => ({ keyword: r.keyword, vendorName: r.vendorName }))
}

async function applyFilters() {
  let startDate: string | null = null
  let endDate: string | null = null

  if (scope.value === 'month') {
    startDate = selectedDate.value.startOf('month').toISODate()!
    endDate = selectedDate.value.endOf('month').toISODate()!
  } else if (scope.value === 'year') {
    startDate = selectedDate.value.startOf('year').toISODate()!
    endDate = selectedDate.value.endOf('year').toISODate()!
  }

  const [txResult, monthly, annual] = await Promise.all([
    api.listTransactions({ startDate, endDate, limit: null }),
    api.periodStats({ groupBy: 'month', excludePeriod: selectedDate.value.toFormat('yyyy-MM') }),
    api.periodStats({ groupBy: 'year', excludePeriod: selectedDate.value.toFormat('yyyy') }),
  ])
  scopeTransactions.value = txResult.rows
  monthlyPeriodStats.value = monthly
  annualPeriodStats.value = annual

  playReveal()
}

async function loadRecurringWindow() {
  const start = DateTime.now().minus({ months: 3 }).startOf('month').toISODate()!
  const result = await api.listTransactions({ startDate: start, endDate: null, limit: null })
  recurringWindowTransactions.value = result.rows
}

async function probeHasAnyTransactions() {
  const result = await api.listTransactions({ limit: 1 })
  hasAnyTransactions.value = result.totalCount > 0
}

onMounted(() => run(async () => {
  await accountsStore.load()
  await Promise.all([probeHasAnyTransactions(), loadRecurringWindow(), loadVendorRules()])
  await applyFilters()
}))

// ─── Where it's going: Fixed / Discretionary totals + typical-period trend ─────

const SPEND_BUCKETS = ['fixed', 'discretionary'] as const

const categoryTotals = computed(() => {
  let fixed = 0
  let discretionary = 0
  let uncategorized = 0
  for (const t of scopeTransactions.value) {
    const f = classifyFlow(t, accountsStore.accounts)
    if (f.isSavings || f.outflow <= 0 || !f.bucket) continue
    if (f.bucket === 'fixed') fixed += f.outflow
    else if (f.bucket === 'discretionary') discretionary += f.outflow
    else uncategorized += f.outflow
  }
  return { fixed, discretionary, uncategorized }
})

const activeMedian = computed(() => {
  if (scope.value === 'month') return computeMedian(monthlyPeriodStats.value)
  if (scope.value === 'year') return computeMedian(annualPeriodStats.value)
  return null
})

// Only show a typical-period comparison with a meaningful baseline, and never
// for "all time" — there's no single period to compare against.
const showComparison = computed(() => scope.value !== 'all' && (activeMedian.value?.periodCount ?? 0) >= 2)

function typicalFor(bucket: 'fixed' | 'discretionary'): number | null {
  if (!showComparison.value) return null
  return activeMedian.value?.breakdown.byCategory.get(bucket) ?? null
}

function trendPctFor(bucket: 'fixed' | 'discretionary'): number | null {
  const typical = typicalFor(bucket)
  return typical == null ? null : pctVsMedian(categoryTotals.value[bucket], typical)
}

// ─── Top merchants ──────────────────────────────────────────────────────────────

const merchantGroups = computed(() => groupByMerchant(scopeTransactions.value, accountsStore.accounts, vendorRules.value))

// ─── Savings opportunities ──────────────────────────────────────────────────────

const recurringCharges = computed(() =>
  detectRecurring(
    recurringWindowTransactions.value,
    accountsStore.accounts,
    { asOf: DateTime.now().toISODate()! },
    vendorRules.value,
  )
)

const categorySpikes = computed(() => {
  if (!showComparison.value) return []
  const typical = activeMedian.value?.breakdown.byCategory
  if (!typical) return []
  return detectCategorySpikes(
    { fixed: categoryTotals.value.fixed, discretionary: categoryTotals.value.discretionary },
    { fixed: typical.get('fixed') ?? 0, discretionary: typical.get('discretionary') ?? 0 },
  )
})

// Recurring charges rank by annualized cost, spikes by their dollar delta this
// period — different scales, but that's the right intuition: a $12/mo
// subscription that's been running all year is a bigger "sure thing" to cut
// than a single elevated month, so it earns the higher spot.
const opportunities = computed<OpportunityItem[]>(() => {
  const ranked: Array<{ impact: number; item: OpportunityItem }> = []

  for (const c of recurringCharges.value) {
    ranked.push({
      impact: c.annualized,
      item: {
        id: `recurring-${c.key}`,
        tone: 'warning',
        icon: 'i-ph-repeat',
        title: c.displayName,
        subtitle: `${c.monthsSeen} of the last 4 months · ${money(c.monthlyAmount)}/mo`,
        trailing: `${money(c.annualized)}/yr`,
        searchTerm: c.searchTerm,
      },
    })
  }

  for (const s of categorySpikes.value) {
    const delta = s.amount - s.typical
    ranked.push({
      impact: delta,
      item: {
        id: `spike-${s.category}`,
        tone: 'error',
        icon: 'i-ph-trend-up',
        title: `${CATEGORY_LABELS[s.category]} spending`,
        subtitle: `${Math.round(s.pct * 100)}% above your typical ${periodWord.value}`,
        trailing: `+${money(delta)}`,
      },
    })
  }

  return ranked.sort((a, b) => b.impact - a.impact).map((r) => r.item)
})

// ─── Uncategorized nudge (data-quality prompt, not a spending fact) ────────────

const uncategorizedTotal = computed(() => categoryTotals.value.uncategorized)

// ─── Drill-down into Transactions ───────────────────────────────────────────────

// Every figure on this page is computed from expense-type transactions within
// the currently selected scope — carry both over so Transactions shows exactly
// what fed the number that was clicked, not a broader set that happens to
// share a search term or category.
function scopeQuery(): Record<string, string> {
  if (scope.value === 'all') return { period: 'all' }
  return { period: scope.value, date: selectedDate.value.toISODate()! }
}

function viewInTransactions(searchTerm: string) {
  router.push({ path: '/transactions', query: { search: searchTerm, types: 'expense', ...scopeQuery() } })
}

function viewUncategorized() {
  router.push({ path: '/transactions', query: { category: 'uncategorized', types: 'expense', ...scopeQuery() } })
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}
</script>

<template>
  <div class="p-6 space-y-6">
    <PageError v-if="error" :message="error" @retry="retry" />

    <template v-else>
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-semibold">Expenses</h1>
        <div v-if="hasAnyTransactions" class="flex items-center gap-3">
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
      </div>

      <!-- No transactions anywhere yet: send the user to Transactions rather than
           duplicating its add/import affordances on a read-only analysis page. -->
      <div v-if="!hasAnyTransactions" class="flex flex-col items-center gap-3 py-24 text-center border border-default rounded-lg">
        <UIcon name="i-ph-chart-pie-slice" class="size-8 text-dimmed" />
        <div>
          <p class="text-sm font-medium text-heading">No transactions yet</p>
          <p class="text-xs text-muted mt-1">Add transactions or import a CSV to see where your money's going.</p>
        </div>
        <UButton size="sm" icon="i-ph-arrow-right" @click="router.push('/transactions')">Go to Transactions</UButton>
      </div>

      <template v-else>
        <!-- Where it's going -->
        <div class="border border-default rounded-lg p-6">
          <p class="text-sm text-muted mb-4">Where it's going — {{ periodLabel }}</p>
          <div class="flex flex-wrap gap-x-12 gap-y-4">
            <div v-for="bucket in SPEND_BUCKETS" :key="bucket" class="shrink-0">
              <p class="text-2xl font-bold tabular-nums leading-none" :class="CATEGORY_TEXT_COLOR[bucket]">
                {{ money(categoryTotals[bucket] * reveal) }}
              </p>
              <p class="text-xs text-muted mt-1.5">{{ CATEGORY_LABELS[bucket] }}</p>
              <p v-if="showComparison && typicalFor(bucket) != null" class="text-xs flex items-center gap-1 mt-0.5">
                <UIcon
                  :name="trendIcon(trendPctFor(bucket))"
                  class="size-3 shrink-0"
                  :class="changeColor('expense', trendPctFor(bucket))"
                />
                <span class="text-dimmed tabular-nums">typ. {{ money(typicalFor(bucket)!) }}</span>
              </p>
            </div>
          </div>

          <p v-if="uncategorizedTotal > 0" class="text-xs text-muted mt-5 pt-4 border-t border-default">
            <button type="button" class="hover:underline text-info" @click="viewUncategorized">
              {{ money(uncategorizedTotal) }} isn't categorized yet
            </button>
            — categorize it on Transactions for a clearer picture.
          </p>
        </div>

        <!-- Top merchants -->
        <div class="border border-default rounded-lg p-6">
          <p class="text-sm text-muted mb-4">Top merchants — {{ periodLabel }}</p>
          <ExpenseMerchantList :merchants="merchantGroups" @select="viewInTransactions" />
        </div>

        <!-- Savings opportunities -->
        <div class="border border-default rounded-lg p-6">
          <p class="text-sm text-muted mb-1">Savings opportunities</p>
          <p class="text-xs text-dimmed mb-4">Recurring charges from the last 4 months, and categories running above typical.</p>
          <ExpenseOpportunities :items="opportunities" @select="viewInTransactions" />
        </div>
      </template>
    </template>
  </div>
</template>
