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

const store = useContributionsStore()
const accountsStore = useAccountsStore()
const fp = useFireProfileStore()
const { error, run, retry } = usePageData()

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

function barColor(pctUsed: number | undefined): string {
  if (pctUsed === undefined) return 'bg-success'
  if (pctUsed > 1) return 'bg-error'
  if (pctUsed >= 0.8) return 'bg-warning'
  return 'bg-success'
}

function barWidth(pctUsed: number | undefined): string {
  if (pctUsed === undefined) return '0%'
  return `${Math.min(pctUsed * 100, 100)}%`
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
}

onMounted(() => run(async () => {
  await Promise.all([accountsStore.load(), fp.load(), store.loadYears()])
  selectedYear.value = DateTime.now().year
  await store.load(selectedYear.value)
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
      <div class="text-3xl font-bold font-mono tabular-nums">{{ money(ytdTotal) }}</div>
      <div class="text-xs text-muted mt-1">across {{ rows.length }} contribution {{ rows.length === 1 ? 'type' : 'types' }}</div>
    </div>

    <!-- Card grid -->
    <div v-if="rows.length" class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <div
        v-for="row in rows"
        :key="row.label"
        class="border border-default rounded-lg p-4 space-y-2"
      >
        <div class="text-xs font-semibold text-muted uppercase tracking-wider">{{ row.label }}</div>
        <div class="text-xl font-bold tabular-nums font-mono">{{ money(row.total) }}</div>

        <template v-if="row.limit !== undefined">
          <div
            role="progressbar"
            :aria-valuenow="Math.round((row.pctUsed ?? 0) * 100)"
            aria-valuemin="0"
            aria-valuemax="100"
            :aria-label="`${row.label}: ${pct(row.pctUsed)} of ${limitLabel(row)}`"
            class="bg-elevated rounded-full h-2 overflow-hidden"
          >
            <div
              class="h-full rounded-full transition-all duration-300"
              :class="barColor(row.pctUsed)"
              :style="{ width: barWidth(row.pctUsed) }"
            />
          </div>
          <div
            v-if="row.pctUsed !== undefined && row.pctUsed > 1"
            class="text-xs font-medium text-error"
          >
            Over limit — excess contributions may be subject to a 6% excise tax
          </div>
          <div v-else class="text-xs text-muted">{{ pct(row.pctUsed) }} of {{ limitLabel(row) }}</div>
        </template>
        <div v-else class="text-xs text-muted">No IRS limit</div>

        <div
          v-if="row.yoyDelta !== undefined"
          class="text-xs"
          :class="row.yoyDelta > 0 ? 'text-success' : row.yoyDelta < 0 ? 'text-muted' : 'text-muted'"
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
    <div v-if="!rows.length" class="py-12 text-center space-y-2">
      <div class="text-sm font-medium">No contributions recorded for {{ selectedYear }}</div>
      <p class="text-sm text-muted max-w-sm mx-auto">
        Contributions are imported automatically from Paychecks or recorded individually in Transactions.
      </p>
    </div>

    <!-- Grouped transaction tables (collapsible) -->
    <div
      v-for="row in rows"
      :key="`group-${row.label}`"
      class="border border-default rounded-lg overflow-hidden"
    >
      <button
        class="w-full bg-elevated px-4 py-2.5 flex justify-between items-center hover:bg-accented/50 transition-colors duration-150 text-left"
        :aria-expanded="expandedGroups.has(row.label)"
        @click="toggleGroup(row.label)"
      >
        <span class="font-medium text-sm">{{ row.label }}</span>
        <div class="flex items-center gap-4">
          <div class="flex gap-4 text-xs text-muted">
            <span>YTD: <strong class="text-default font-mono tabular-nums">{{ money(row.total) }}</strong></span>
            <span v-if="row.limit !== undefined">Limit: <strong class="text-default font-mono tabular-nums">{{ money(row.limit) }}</strong></span>
            <span
              v-if="row.pctUsed !== undefined"
              :class="row.pctUsed > 1 ? 'text-error font-medium' : ''"
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
</template>
