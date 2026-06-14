<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { useContributionsStore } from '../stores/contributions'
import { useAccountsStore } from '../stores/accounts'
import { useFireProfileStore } from '../stores/fireProfile'
import { buildContributionRows, type ContributionRow } from '../lib/contributions/index'
import { resolveYearLimits } from '../lib/contributions/irsLimits'

const store = useContributionsStore()
const accountsStore = useAccountsStore()
const fp = useFireProfileStore()

const selectedYear = ref<number>(DateTime.now().year)

const resolved = computed(() => resolveYearLimits(selectedYear.value))

const rows = computed<ContributionRow[]>(() => {
  if (!fp.profile) return []
  return buildContributionRows(
    store.txns,
    accountsStore.accounts,
    selectedYear.value,
    fp.profile.currentAge,
    (fp.profile.hsaCoverage as 'self' | 'family') ?? 'self',
    resolved.value.limits,
  )
})

const ytdTotal = computed(() => rows.value.reduce((s, r) => s + r.total, 0))

function accountName(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

function accountType(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.type ?? ''
}

// Transactions belonging to a given row's account types, for the selected year.
function rowTxns(row: ContributionRow) {
  return store.txns.filter(
    (t) =>
      t.date.startsWith(String(selectedYear.value)) &&
      row.accountTypes.includes(accountType(t.accountId)),
  )
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

function pct(n: number | undefined): string {
  return n === undefined ? '—' : `${(n * 100).toFixed(0)}%`
}

// Amber when near the cap (80-99%) or over it (>100%); green when on track or exactly maxed.
function barColor(pctUsed: number | undefined): string {
  if (pctUsed === undefined) return 'bg-green-500'
  if (pctUsed > 1) return 'bg-amber-500'
  if (pctUsed >= 0.8 && pctUsed < 1) return 'bg-amber-500'
  return 'bg-green-500'
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

async function onYearChange(year: unknown) {
  const y = Number(year)
  selectedYear.value = y
  await store.load(y)
}

onMounted(async () => {
  await Promise.all([accountsStore.load(), fp.load(), store.loadYears()])
  selectedYear.value = DateTime.now().year
  await store.load(selectedYear.value)
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Contributions</h1>
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
      :description="`Update irsLimits.ts when ${selectedYear} limits are announced.`"
    />

    <div class="flex gap-6 text-sm">
      <span>YTD Total: <strong>{{ money(ytdTotal) }}</strong></span>
      <span class="text-muted">{{ rows.length }} account types</span>
    </div>

    <!-- Card grid -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <div
        v-for="row in rows"
        :key="row.label"
        class="border border-default rounded-lg p-4 space-y-2"
      >
        <div class="text-sm font-medium">{{ row.label }}</div>
        <div class="text-xl font-bold tabular-nums">{{ money(row.total) }}</div>
        <template v-if="row.limit !== undefined">
          <div class="bg-elevated rounded-full h-1.5 overflow-hidden">
            <div
              class="h-full rounded-full"
              :class="barColor(row.pctUsed)"
              :style="{ width: barWidth(row.pctUsed) }"
            />
          </div>
          <div class="text-xs text-muted">{{ pct(row.pctUsed) }} of {{ limitLabel(row) }}</div>
        </template>
        <div v-else class="text-xs text-muted">No IRS limit</div>
        <div
          v-if="row.yoyDelta !== undefined"
          class="text-xs"
          :class="row.yoyDelta > 0 ? 'text-green-600' : row.yoyDelta < 0 ? 'text-red-600' : 'text-muted'"
        >
          {{ row.yoyDelta > 0 ? '+' : '' }}{{ money(row.yoyDelta) }} vs {{ selectedYear - 1 }}
        </div>
        <div
          v-if="row.breakdown"
          class="text-xs text-muted pt-2 border-t border-default/50"
        >
          {{ row.breakdown.map((b) => `${b.label}: ${money(b.total)}`).join(' · ') }}
        </div>
      </div>
    </div>

    <p v-if="!rows.length" class="text-muted text-sm">
      No contributions recorded for {{ selectedYear }}.
    </p>

    <!-- Grouped transaction table -->
    <div v-for="row in rows" :key="`group-${row.label}`" class="border border-default rounded-lg overflow-hidden">
      <div class="bg-elevated px-4 py-2 flex justify-between items-center">
        <span class="font-medium text-sm">{{ row.label }}</span>
        <div class="flex gap-4 text-xs text-muted">
          <span>YTD: <strong class="text-default">{{ money(row.total) }}</strong></span>
          <span v-if="row.limit !== undefined">Limit: <strong class="text-default">{{ money(row.limit) }}</strong></span>
          <span v-if="row.pctUsed !== undefined">{{ pct(row.pctUsed) }}</span>
        </div>
      </div>
      <table class="w-full text-sm">
        <thead class="text-left text-muted border-t border-default">
          <tr>
            <th class="px-4 py-2 font-normal w-28">Date</th>
            <th class="py-2 font-normal">Description</th>
            <th class="py-2 font-normal">Account</th>
            <th class="py-2 font-normal">Source</th>
            <th class="px-4 py-2 font-normal text-right">Amount</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="t in rowTxns(row)"
            :key="t.id"
            class="border-t border-default/50"
          >
            <td class="px-4 py-2 text-muted w-28">{{ t.date }}</td>
            <td class="py-2">{{ t.description }}</td>
            <td class="py-2 text-muted">{{ accountName(t.accountId) }}</td>
            <td class="py-2 text-muted text-xs">{{ importLabel(t.importSource) }}</td>
            <td class="px-4 py-2 text-right tabular-nums">{{ money(t.amount) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
