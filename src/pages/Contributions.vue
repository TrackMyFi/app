<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { useContributionsStore } from '../stores/contributions'
import { useAccountsStore } from '../stores/accounts'
import { useFireProfileStore } from '../stores/fireProfile'
import { buildContributionRows, type ContributionRow } from '../lib/contributions/index'
import { resolveYearLimits } from '../lib/contributions/irsLimits'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'
import { useReveal } from '../composables/useReveal'

const store = useContributionsStore()
const accountsStore = useAccountsStore()
const fp = useFireProfileStore()
const { error, run, retry } = usePageData()

// Drives the count-up reveal: figures tick up from zero whenever data lands.
const { progress: reveal, play: playReveal } = useReveal()

const selectedYear = ref<number>(DateTime.now().year)

const resolved = computed(() => resolveYearLimits(selectedYear.value))

const rows = computed<ContributionRow[]>(() => {
  if (!fp.profile) return []
  return buildContributionRows(
    store.txns,
    accountsStore.accounts,
    selectedYear.value,
    fp.currentAge,
    (fp.profile.hsaCoverage as 'self' | 'family') ?? 'self',
    resolved.value.limits,
  )
})

const ytdTotal = computed(() => rows.value.reduce((s, r) => s + r.total, 0))

// Collapsed by default — cards above give the summary; transactions are detail-on-demand.
const expandedGroups = ref(new Set<string>(rows.value.map((v) => v.label)))

function toggleGroup(label: string) {
  if (expandedGroups.value.has(label)) {
    expandedGroups.value.delete(label)
  } else {
    expandedGroups.value.add(label)
  }
  // Trigger reactivity on the Set
  expandedGroups.value = new Set(expandedGroups.value)
}

function accountName(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

function accountType(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.type ?? ''
}

function rowTxns(row: ContributionRow) {
  return store.txns.filter(
    (t) =>
      t.date.startsWith(String(selectedYear.value)) &&
      row.accountTypes.includes(accountType(t.transferAccountId ?? t.accountId)),
  )
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

function pct(n: number | undefined): string {
  return n === undefined ? '—' : `${(n * 100).toFixed(0)}%`
}

// A small epsilon separates "hit your number exactly" (celebrate) from a true
// over-contribution that risks the excise tax (caution).
const OVER_EPSILON = 1.0005

function isMaxed(row: ContributionRow): boolean {
  return (
    row.limit !== undefined &&
    row.pctUsed !== undefined &&
    row.pctUsed >= 1 &&
    row.pctUsed <= OVER_EPSILON
  )
}

function isOver(pctUsed: number | undefined): boolean {
  return pctUsed !== undefined && pctUsed > OVER_EPSILON
}

// Dollars left before this account is maxed — the gap a FIRE user actually
// wants surfaced. Undefined when there's no limit or it's already funded.
function toGo(row: ContributionRow): number | undefined {
  if (row.limit === undefined) return undefined
  const remaining = row.limit - row.total
  return remaining > 0 ? remaining : undefined
}

const limitedRows = computed(() => rows.value.filter((r) => r.limit !== undefined))
const allMaxed = computed(
  () => limitedRows.value.length > 0 && limitedRows.value.every(isMaxed),
)

// Five emerald motes for the rare all-maxed celebration. Fixed positions/delays
// so the sparkle reads as composed, not random.
const motes = [
  { left: '14%', delay: '0ms' },
  { left: '32%', delay: '190ms' },
  { left: '50%', delay: '80ms' },
  { left: '68%', delay: '270ms' },
  { left: '86%', delay: '140ms' },
]

// The climb is emerald; only a genuine over-contribution turns red.
function barColor(pctUsed: number | undefined): string {
  return isOver(pctUsed) ? 'bg-error' : 'bg-primary'
}

// Width is scaled by the reveal multiplier so bars grow from zero on load.
function barWidth(pctUsed: number | undefined): string {
  if (pctUsed === undefined) return '0%'
  return `${Math.min(pctUsed * 100, 100) * reveal.value}%`
}

function limitLabel(row: ContributionRow): string {
  if (row.limit === undefined) return 'No IRS limit'
  const base = money(row.limit)
  if (row.catchUpAmount) return `${base} (incl. ${money(row.catchUpAmount)} catch-up)`
  return base
}

function importLabel(source: string): string {
  return source === 'paycheck' ? 'via Paycheck' : 'Manual'
}

const contributionColumns = [
  { accessorKey: 'date', header: 'Date' },
  { accessorKey: 'description', header: 'Description' },
  { id: 'account', header: 'Account' },
  { id: 'source', header: 'Source' },
  { id: 'amount', header: 'Amount', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
]

async function onYearChange(year: unknown) {
  const y = Number(year)
  selectedYear.value = y
  expandedGroups.value = new Set()
  await store.load(y)
  playReveal()
}

onMounted(() => run(async () => {
  await Promise.all([accountsStore.load(), fp.load(), store.loadYears()])
  selectedYear.value = DateTime.now().year
  await store.load(selectedYear.value)
  playReveal()
}))
</script>

<template>
  <div class="p-6 space-y-6">
    <PageError v-if="error" :message="error" @retry="retry" />

    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">Contributions</h1>
      <USelect
        :model-value="selectedYear"
        :items="store.years.map((y) => ({ label: String(y), value: y }))"
        class="w-28"
        @update:model-value="onYearChange"
      />
    </div>

    <UAlert
      v-if="resolved.estimated"
      color="warning"
      variant="soft"
      icon="i-ph-info"
      :title="`IRS limits estimated from ${resolved.estimatedFrom}`"
      :description="`Official ${selectedYear} limits haven't been published yet. These figures are projected from prior-year data.`"
    />

    <!-- YTD aggregate stat -->
    <div v-if="rows.length">
      <div class="text-xs font-semibold text-muted uppercase tracking-wider mb-1">{{ selectedYear }} Total</div>
      <div class="text-3xl font-bold font-mono tabular-nums text-highlighted">{{ money(ytdTotal * reveal) }}</div>
      <div class="text-xs text-muted mt-1">across {{ rows.length }} contribution {{ rows.length === 1 ? 'type' : 'types' }}</div>
    </div>

    <!-- The rare all-maxed milestone: every limited account fully funded. -->
    <div
      v-if="allMaxed"
      :key="`allmaxed-${selectedYear}`"
      class="tmfi-rise relative overflow-hidden flex items-center gap-2.5 rounded-lg border border-primary/40 bg-primary/[0.04] px-4 py-3"
    >
      <UIcon name="i-ph-seal-check" class="w-5 h-5 text-primary shrink-0" />
      <span class="text-sm font-medium text-primary">
        Every tax-advantaged account maxed for {{ selectedYear }}.
      </span>
      <span
        v-for="(m, i) in motes"
        :key="i"
        class="tmfi-mote"
        :style="{ left: m.left, animationDelay: m.delay }"
        aria-hidden="true"
      />
    </div>

    <!-- Card grid -->
    <div v-if="rows.length" class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <div
        v-for="(row, i) in rows"
        :key="`${selectedYear}-${row.label}`"
        class="tmfi-rise relative overflow-hidden rounded-lg border p-4 space-y-2 transition-colors duration-200"
        :class="isMaxed(row) ? 'border-primary/40 bg-primary/[0.04]' : 'border-default'"
        :style="{ animationDelay: `${i * 55}ms` }"
      >
        <div class="flex items-start justify-between gap-2">
          <div class="text-xs font-semibold text-muted uppercase tracking-wider">{{ row.label }}</div>
          <span
            v-if="isMaxed(row)"
            class="inline-flex items-center gap-1 text-[0.65rem] font-bold uppercase tracking-wider text-primary shrink-0"
          >
            <svg class="tmfi-check w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <path
                d="M5 13l4 4L19 7"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            Maxed
          </span>
        </div>

        <div class="text-xl font-bold tabular-nums font-mono">{{ money(row.total * reveal) }}</div>

        <template v-if="row.limit !== undefined">
          <div
            role="progressbar"
            :aria-valuenow="Math.round((row.pctUsed ?? 0) * 100)"
            aria-valuemin="0"
            aria-valuemax="100"
            :aria-label="`${row.label}: ${pct(row.pctUsed)} of ${limitLabel(row)}`"
            class="relative bg-elevated rounded-full h-2 overflow-hidden"
          >
            <div
              class="relative h-full rounded-full transition-[width] duration-300 ease-out"
              :class="barColor(row.pctUsed)"
              :style="{ width: barWidth(row.pctUsed) }"
            >
              <span v-if="isMaxed(row)" class="tmfi-sheen" aria-hidden="true" />
            </div>
          </div>
          <div v-if="isOver(row.pctUsed)" class="text-xs font-medium text-error">
            Over limit — excess contributions may be subject to a 6% excise tax
          </div>
          <div v-else-if="isMaxed(row)" class="text-xs font-medium text-primary">
            Fully funded for {{ selectedYear }}
          </div>
          <div v-else class="text-xs text-muted">
            <span class="font-mono tabular-nums font-medium text-default">{{ money(toGo(row) ?? 0) }}</span>
            to go · {{ pct(row.pctUsed) }} of {{ limitLabel(row) }}
          </div>
        </template>
        <div v-else class="text-xs text-muted">No IRS limit</div>

        <div
          v-if="row.yoyDelta !== undefined"
          class="text-xs"
          :class="row.yoyDelta > 0 ? 'text-success' : 'text-muted'"
        >
          {{ row.yoyDelta > 0 ? '+' : '' }}<span class="font-mono tabular-nums">{{ money(row.yoyDelta) }}</span> vs {{ selectedYear - 1 }}
        </div>

        <div
          v-if="row.breakdown"
          class="text-xs text-muted pt-2 border-t border-default/50 space-y-0.5"
        >
          <div v-for="b in row.breakdown" :key="b.type" class="flex justify-between">
            <span>{{ b.label }}</span>
            <span class="font-mono tabular-nums">{{ money(b.total) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="!rows.length"
      class="tmfi-rise border border-dashed border-default rounded-lg p-8 text-center"
    >
      <UIcon name="i-ph-piggy-bank" class="w-8 h-8 text-muted mx-auto mb-3" />
      <p class="text-sm font-medium mb-1">No contributions recorded for {{ selectedYear }}</p>
      <p class="text-sm text-muted max-w-sm mx-auto">
        Contributions are imported automatically from Paychecks or recorded individually in Transactions.
      </p>
    </div>

    <!-- Grouped transaction tables (collapsible): the detail behind the cards -->
    <div v-if="rows.length" class="space-y-3">
      <div class="text-xs font-semibold text-muted uppercase tracking-wider">Transactions</div>
      <div
        v-for="(row, i) in rows"
        :key="`group-${selectedYear}-${row.label}`"
        class="tmfi-rise border border-default rounded-lg overflow-hidden"
        :style="{ animationDelay: `${i * 45}ms` }"
      >
        <button
          class="w-full bg-elevated px-4 py-2.5 flex justify-between items-center hover:bg-accented/50 focus-visible:outline-none focus-visible:bg-accented/50 focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary/40 transition-colors duration-150 text-left"
          :aria-expanded="expandedGroups.has(row.label)"
          @click="toggleGroup(row.label)"
        >
          <span class="flex items-center gap-1.5 font-medium text-sm">
            {{ row.label }}
            <UIcon v-if="isMaxed(row)" name="i-ph-seal-check" class="w-4 h-4 text-primary" />
          </span>
          <div class="flex items-center gap-4">
            <div class="flex gap-4 text-xs text-muted">
              <span>YTD: <strong class="text-default font-mono tabular-nums">{{ money(row.total) }}</strong></span>
              <span v-if="row.limit !== undefined">Limit: <strong class="text-default font-mono tabular-nums">{{ money(row.limit) }}</strong></span>
              <span
                v-if="row.pctUsed !== undefined"
                :class="isOver(row.pctUsed) ? 'text-error font-medium' : isMaxed(row) ? 'text-primary font-medium' : ''"
              >{{ pct(row.pctUsed) }}</span>
            </div>
            <UIcon
              :name="expandedGroups.has(row.label) ? 'i-ph-caret-up' : 'i-ph-caret-down'"
              class="text-muted w-4 h-4 shrink-0"
            />
          </div>
        </button>

        <div v-if="expandedGroups.has(row.label)">
          <UTable :data="rowTxns(row)" :columns="contributionColumns">
            <template #account-cell="{ row: txnRow }">
              <span class="text-muted">{{ accountName(txnRow.original.accountId) }}</span>
            </template>
            <template #source-cell="{ row: txnRow }">
              <span class="text-muted text-xs">{{ importLabel(txnRow.original.importSource) }}</span>
            </template>
            <template #amount-cell="{ row: txnRow }">
              <span class="font-mono tabular-nums">{{ money(txnRow.original.amount) }}</span>
            </template>
          </UTable>
        </div>
      </div>
    </div>
  </div>
</template>
