<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { DateTime } from 'luxon'
import { useToast } from '@nuxt/ui/composables'
import { usePaychecksStore } from '../stores/paychecks'
import { listPaychecks } from '../lib/api/paychecks'
import { labelForPayPeriod } from '../lib/paychecks/constants'
import { useAccountsStore } from '../stores/accounts'
import { paycheckBreakdown } from '../lib/paychecks/index'
import { projectYearEnd } from '../lib/paychecks/projection'
import PaycheckForm from '../components/PaycheckForm.vue'
import MonthPicker from '../components/MonthPicker.vue'
import type { Paycheck } from '../lib/types/Paycheck'
import { confirm } from '@tauri-apps/plugin-dialog'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'
import { useReveal } from '../composables/useReveal'

const store = usePaychecksStore()
const accountsStore = useAccountsStore()
const toast = useToast()
const { loading, error, run, retry } = usePageData()

// Income materializes rather than just appearing: the summary figures tick up
// and the take-home bar grows from zero whenever paychecks land or the filter
// changes — the same count-up vocabulary as Budget and Contributions.
const { progress: reveal, play: playReveal } = useReveal()

const isModalOpen = ref(false)
const editing = ref<Paycheck | null>(null)
const copySource = ref<Paycheck | null>(null)

function openAdd() { editing.value = null; copySource.value = null; isModalOpen.value = true }
function openEdit(p: Paycheck) { editing.value = p; copySource.value = null; isModalOpen.value = true }
function openCopy(p: Paycheck) { editing.value = null; copySource.value = p; isModalOpen.value = true }
function onSaved(close = true) { if (close) isModalOpen.value = false }

watch(isModalOpen, (open) => {
  if (!open) {
    editing.value = null
    copySource.value = null
  }
})

const removingId = ref<number | null>(null)

async function removeRow(p: Paycheck) {
  const ok = await confirm(`Delete paycheck from "${p.employer}" on ${p.payDate}?`, {
    title: 'Delete paycheck',
  })
  if (!ok) return
  removingId.value = p.id
  try {
    await store.remove(p.id)
  } catch (err) {
    toast.add({ title: 'Failed to delete paycheck', description: String(err), color: 'error' })
  } finally {
    removingId.value = null
  }
}

// Year-by-year is the natural grain for income questions ("how much this
// year?", "how does it compare to last?"), so the page defaults to the current
// year with arrows to step back — same scope vocabulary as Transactions, minus
// the month grain that paychecks don't need.
type Scope = 'year' | 'all'
const scope = ref<Scope>('year')
const selectedDate = ref(DateTime.now().startOf('year'))
const yearLabel = computed(() => selectedDate.value.toFormat('yyyy'))

function setScope(s: Scope) {
  scope.value = s
  applyFilters()
}

function onYearChange(dt: DateTime) {
  selectedDate.value = dt.startOf('year')
  applyFilters()
}

const employerSearch = ref('')

async function applyFilters() {
  const inYear = scope.value === 'year'
  await store.setFilter({
    startDate: inYear ? selectedDate.value.toISODate() : null,
    endDate: inYear ? selectedDate.value.endOf('year').toISODate() : null,
    employer: employerSearch.value || null,
  })
  playReveal()
}

async function clearFilters() {
  scope.value = 'all'
  employerSearch.value = ''
  await store.setFilter({ startDate: null, endDate: null, employer: null })
  playReveal()
}

const breakdown = computed(() => paycheckBreakdown(store.paychecks))

// Bonuses, RSU vests, and other one-offs carry their own flag on the paycheck
// (payPeriod === 'irregular'), so "how much of this was windfall?" is just the
// breakdown re-run on that slice.
const irregularBreakdown = computed(() =>
  paycheckBreakdown(store.paychecks.filter((p) => p.payPeriod === 'irregular')),
)

// End-of-year projection only makes sense while the selected year is still in
// progress — a finished year's actuals ARE its year-end figures.
const projection = computed(() => {
  const now = DateTime.now()
  if (scope.value !== 'year' || selectedDate.value.year !== now.year) return null
  return projectYearEnd(store.paychecks, now)
})

const hasPaychecks = computed(() => store.paychecks.length > 0)

// The year scope is a filter that's always on, so "no rows" alone can't tell a
// brand-new user from one whose paychecks all live in other years. Track
// existence separately: any non-empty page proves it; an empty page triggers
// one unfiltered recheck (covers deleting the last paycheck too).
const hasAnyPaychecks = ref(false)
watch(() => store.paychecks, async (rows) => {
  if (rows.length > 0) hasAnyPaychecks.value = true
  else hasAnyPaychecks.value = (await listPaychecks({})).length > 0
})

// True first run: nothing recorded anywhere. Earns the teaching empty state
// rather than a bare "no rows" line.
const isFirstRun = computed(() => !hasAnyPaychecks.value && !loading.value)
// Filter that matched nothing — the data exists, it's just hidden right now.
const isFilteredEmpty = computed(() => !hasPaychecks.value && hasAnyPaychecks.value && !loading.value)

// Bar segments. Net and withheld (gross − net) are the two figures that are
// always real, so the take-home segment is anchored to the headline rate and
// the withheld remainder is divided between taxes and deductions by their
// ratio. The fields are entered independently, so taxes + deductions needn't
// equal withheld — anchoring this way keeps the bar exactly full (no clipping,
// no gap) while the legend still carries the exact dollar figures. Each share
// is grown by the reveal so the whole split sweeps out from zero together.
const netShare = computed(() => {
  const g = breakdown.value.totalGross
  return g > 0 ? Math.min(breakdown.value.totalNet / g, 1) : 0
})
const withheldShare = computed(() => Math.max(1 - netShare.value, 0))
const taxShare = computed(() => {
  const denom = breakdown.value.totalTaxes + breakdown.value.totalDeductions
  return denom > 0 ? withheldShare.value * (breakdown.value.totalTaxes / denom) : 0
})
const dedShare = computed(() => {
  const denom = breakdown.value.totalTaxes + breakdown.value.totalDeductions
  return denom > 0 ? withheldShare.value * (breakdown.value.totalDeductions / denom) : 0
})
const netWidth = computed(() => `${netShare.value * 100 * reveal.value}%`)
const taxWidth = computed(() => `${taxShare.value * 100 * reveal.value}%`)
const dedWidth = computed(() => `${dedShare.value * 100 * reveal.value}%`)
const takeHomePct = computed(() =>
  (breakdown.value.takeHomeRate * reveal.value * 100).toLocaleString('en-US', {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }),
)

const columns = [
  { accessorKey: 'payDate', header: 'Date' },
  { accessorKey: 'employer', header: 'Employer' },
  { id: 'period', header: 'Period' },
  { id: 'gross', header: 'Gross', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'net', header: 'Net', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'federal', header: 'Federal', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'ssMedicare', header: 'SS + Medicare', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'actions', header: '', meta: { class: { td: 'text-right' } } },
]

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

onMounted(() => run(async () => {
  await accountsStore.load() // pre-populates store for PaycheckForm account dropdowns
  employerSearch.value = store.filter.employer ?? ''
  await applyFilters() // seeds the default current-year scope and loads
}))
</script>

<template>
  <div class="p-6 space-y-4">
    <PageError v-if="error" :message="error" @retry="retry" />

    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Paychecks</h1>
      <UButton icon="i-ph-plus" @click="openAdd">Add paycheck</UButton>
    </div>

    <!-- First run: nothing recorded yet. Teach the page instead of an empty grid. -->
    <div
      v-if="!error && isFirstRun"
      class="tmfi-rise flex flex-col items-center justify-center text-center border border-dashed border-default rounded-lg px-6 py-16"
    >
      <div class="size-12 rounded-full bg-primary/10 text-primary flex items-center justify-center mb-4">
        <UIcon name="i-ph-receipt" class="size-6" />
      </div>
      <h2 class="text-lg font-semibold">Track your income</h2>
      <p class="text-sm text-muted mt-1 max-w-sm text-balance">
        Add your paychecks to see how much you earn, how much you keep, and what goes to taxes over time.
      </p>
      <UButton icon="i-ph-plus" class="mt-5" @click="openAdd">Add your first paycheck</UButton>
    </div>

    <template v-else-if="!error">
      <div class="flex flex-wrap gap-2 items-center">
        <MonthPicker
          v-if="scope === 'year'"
          :model-value="selectedDate"
          mode="year"
          @update:model-value="onYearChange"
        />
        <div class="flex gap-1">
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
        <UInput v-model="employerSearch" placeholder="Search employer" class="w-44 ml-auto" @keyup.enter="applyFilters" />
        <UButton @click="applyFilters">Apply</UButton>
        <UButton variant="ghost" @click="clearFilters">Clear</UButton>
      </div>

      <!-- Income summary: of every gross dollar, what you kept vs what was
           withheld. Net is the one earned emerald — it's the cash that funds
           independence. Figures tick up and the bar sweeps out on reveal. -->
      <section v-if="hasPaychecks" class="tmfi-rise border border-default rounded-lg p-5">
        <div class="flex flex-wrap items-end justify-between gap-4">
          <div>
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Net take-home</p>
            <p class="text-2xl font-bold font-mono tabular-nums text-primary mt-0.5">{{ money(breakdown.totalNet * reveal) }}</p>
            <p class="text-xs text-muted mt-1">
              of {{ money(breakdown.totalGross * reveal) }} gross · {{ breakdown.count }} paycheck{{ breakdown.count === 1 ? '' : 's' }} · {{ scope === 'year' ? yearLabel : 'all time' }}
            </p>
          </div>
          <div v-if="irregularBreakdown.count > 0">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Irregular pay</p>
            <p class="text-2xl font-bold font-mono tabular-nums mt-0.5">{{ money(irregularBreakdown.totalNet * reveal) }}</p>
            <p class="text-xs text-muted mt-1">
              of {{ money(irregularBreakdown.totalGross * reveal) }} gross · {{ irregularBreakdown.count }} payment{{ irregularBreakdown.count === 1 ? '' : 's' }}
            </p>
          </div>
          <div v-if="projection && projection.remainingCount > 0">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Projected {{ yearLabel }}</p>
            <p class="text-2xl font-bold font-mono tabular-nums mt-0.5">≈{{ money(projection.projectedNet * reveal) }}</p>
            <p class="text-xs text-muted mt-1">
              net of ≈{{ money(projection.projectedGross * reveal) }} gross · {{ projection.remainingCount }} paycheck{{ projection.remainingCount === 1 ? '' : 's' }} to come
            </p>
          </div>
          <div class="text-right">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Take-home rate</p>
            <p class="text-2xl font-bold font-mono tabular-nums mt-0.5">{{ takeHomePct }}%</p>
            <p class="text-xs text-muted mt-1">kept per gross dollar</p>
          </div>
        </div>

        <div
          class="mt-4 flex h-2.5 rounded-full overflow-hidden bg-elevated"
          role="img"
          :aria-label="`Gross pay split: ${takeHomePct}% take-home, the rest withheld for taxes and deductions`"
        >
          <div class="h-full bg-primary transition-[width] duration-500 ease-out" :style="{ width: netWidth }" />
          <div class="h-full bg-current text-muted transition-[width] duration-500 ease-out" :style="{ width: taxWidth }" />
          <div class="h-full bg-current text-dimmed transition-[width] duration-500 ease-out" :style="{ width: dedWidth }" />
        </div>

        <div class="mt-3 flex flex-wrap gap-x-6 gap-y-1 text-xs">
          <span class="inline-flex items-center gap-1.5">
            <span class="size-2 rounded-full bg-primary" />
            <span class="text-muted">Take-home</span>
            <span class="font-mono tabular-nums font-medium text-default">{{ money(breakdown.totalNet * reveal) }}</span>
          </span>
          <span v-if="breakdown.totalTaxes > 0" class="inline-flex items-center gap-1.5">
            <span class="size-2 rounded-full bg-current text-muted" />
            <span class="text-muted">Taxes</span>
            <span class="font-mono tabular-nums font-medium text-default">{{ money(breakdown.totalTaxes * reveal) }}</span>
          </span>
          <span v-if="breakdown.totalDeductions > 0" class="inline-flex items-center gap-1.5">
            <span class="size-2 rounded-full bg-current text-dimmed" />
            <span class="text-muted">Deductions</span>
            <span class="font-mono tabular-nums font-medium text-default">{{ money(breakdown.totalDeductions * reveal) }}</span>
          </span>
        </div>
      </section>

      <!-- Filter matched nothing — the data exists, it's just hidden. -->
      <div v-if="isFilteredEmpty" class="border border-default rounded-lg py-12 text-center">
        <p class="text-sm text-muted">
          {{ scope === 'year' && !store.filter.employer ? `No paychecks recorded in ${yearLabel}.` : 'No paychecks match these filters.' }}
        </p>
        <UButton variant="ghost" size="sm" class="mt-2" @click="clearFilters">View all time</UButton>
      </div>

      <div v-else class="border border-default rounded-lg overflow-hidden">
        <UTable :data="store.paychecks" :columns="columns" empty="No paychecks yet.">
          <template #period-cell="{ row }">{{ labelForPayPeriod(row.original.payPeriod) }}</template>
          <template #gross-cell="{ row }">{{ money(row.original.grossAmount) }}</template>
          <template #net-cell="{ row }">{{ money(row.original.netAmount) }}</template>
          <template #federal-cell="{ row }">{{ money(row.original.federalTax) }}</template>
          <template #ssMedicare-cell="{ row }">{{ money(row.original.socialSecurityTax + row.original.medicareTax) }}</template>
          <template #actions-cell="{ row }">
            <UButton size="xs" variant="ghost" icon="i-ph-copy" @click="openCopy(row.original)" />
            <UButton size="xs" variant="ghost" icon="i-ph-pencil" @click="openEdit(row.original)" />
            <UButton size="xs" variant="ghost" color="error" icon="i-ph-trash" :loading="removingId === row.original.id" :disabled="removingId !== null" @click="removeRow(row.original)" />
          </template>
        </UTable>
      </div>
    </template>

    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit paycheck' : copySource ? 'Copy paycheck' : 'Add paycheck'" class="lg:w-[900px] max-w-full">
      <template #body>
        <PaycheckForm :editing="editing" :copy-from="copySource" @saved="onSaved" />
      </template>
    </UModal>
  </div>
</template>
