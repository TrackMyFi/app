<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { useBudgetStore } from '../stores/budget'
import { useAccountsStore } from '../stores/accounts'
import { usePageData } from '../composables/usePageData'
import { useReveal } from '../composables/useReveal'
import CurrencyInput from '../components/CurrencyInput.vue'
import MonthPicker from '../components/MonthPicker.vue'
import PageError from '../components/PageError.vue'
import BudgetSectionCard from '../components/BudgetSectionCard.vue'

const store = useBudgetStore()
const accountsStore = useAccountsStore()
const toast = useToast()
const { loading, error, run, retry } = usePageData()

// Drives the count-up reveal: the whole equation ticks into place from zero
// whenever a month's data lands, so the budget feels like it computes in front
// of you rather than just appearing.
const { progress: reveal, play: playReveal } = useReveal()

const editingTarget = ref(false)
const targetInput = ref<number | null>(null)
const savingTarget = ref(false)

const selectedDate = ref<DateTime>(DateTime.now().startOf('month'))

function formatMonth(year: number, month: number): string {
  return DateTime.fromObject({ year, month }).toFormat('MMMM yyyy')
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

async function onMonthChange(dt: DateTime) {
  selectedDate.value = dt
  await run(() => store.load(dt.year, dt.month))
  playReveal()
}

type BudgetSection = 'income' | 'savings' | 'fixed' | 'discretionary'
const allSections: BudgetSection[] = ['income', 'savings', 'fixed', 'discretionary']

// "At a glance" mode: the active section's card stays on top, and the
// remaining sections expand below the toggle as their own cards.
const showAllSections = ref(false)

// The other three sections, in formula order, shown when expanded.
const remainingSections = computed(() =>
  allSections.filter((s) => s !== store.activeSection),
)

function setActiveSection(section: BudgetSection) {
  store.activeSection = section
}

function openTargetEdit() {
  targetInput.value = store.target?.savingsTarget ?? null
  editingTarget.value = true
}

function cancelTargetEdit() {
  editingTarget.value = false
  targetInput.value = null
}

async function saveTarget() {
  if (targetInput.value === null || targetInput.value < 0) return
  savingTarget.value = true
  try {
    await store.setTarget(targetInput.value)
    toast.add({ title: 'Budget target saved', color: 'success' })
    editingTarget.value = false
    targetInput.value = null
  } catch (err) {
    toast.add({ title: 'Failed to save target', description: String(err), color: 'error' })
  } finally {
    savingTarget.value = false
  }
}

// The income table normally hides paycheck deposit rows (paycheck income
// lives in the summary figures), but a deposit carried in from last month's
// month-end paycheck would otherwise be invisible — surface it, notated.
const incomeTransactions = computed(() => {
  if (!store.summary) return []
  const carriedDeposits = store.carriedIn.filter((t) => t.type === 'income' && !t.isContribution)
  return [...carriedDeposits, ...store.summary.income.transactions]
})

// Newest first so fresh activity tops the table; carried-in rows from last
// month are the oldest by real date and naturally settle at the bottom.
function sectionTransactions(section: BudgetSection) {
  if (!store.summary) return []
  const rows = section === 'income'
    ? incomeTransactions.value
    : store.summary[section].transactions
  return [...rows].sort((a, b) => b.date.localeCompare(a.date) || b.id - a.id)
}

// ─── Month-end paycheck attribution ──────────────────────────────────────────

const monthStartIso = computed(() => selectedDate.value.toISODate()!)
const carriedFromLabel = computed(() => selectedDate.value.minus({ months: 1 }).toFormat('MMM'))

const savingsSubLabel = computed(() => {
  if (editingTarget.value) return null
  if (!store.target) return 'set target'
  if (store.target.isInherited) {
    const from = formatMonth(store.target.sourceYear, store.target.sourceMonth)
    return `from ${from}`
  }
  return `target ${money(store.target.savingsTarget)}`
})


const discretionaryRemaining = computed(() => {
  if (!store.summary) return 0
  return store.summary.freeMoneyRemaining
})

// "Paid yourself first": you've met or beaten your own savings target this
// month. Tied to the real figures (not the animated reveal) so the milestone
// doesn't flicker on while the number counts up. Guards against a 0 target
// reading as "met" the moment any saving happens.
const savingsMet = computed(
  () =>
    !!store.summary &&
    !!store.target &&
    store.target.savingsTarget > 0 &&
    store.summary.savings.total >= store.target.savingsTarget,
)

// Free Money is the result of the equation — the room you have to spend this
// month. The envelope gauge turns the abstract "$X remaining" into something
// you can feel filling up: discretionary spending eats into the envelope, and
// the headroom that's left glows emerald until you cross the line.
const freeMoney = computed(() => store.summary?.freeMoney ?? 0)
const discretionarySpent = computed(() => store.summary?.discretionary.total ?? 0)
const hasEnvelope = computed(() => freeMoney.value > 0)
const overBudget = computed(() => discretionaryRemaining.value < 0)
const showEnvelope = computed(() => hasEnvelope.value || discretionarySpent.value > 0)

const spentRatio = computed(() => {
  if (!hasEnvelope.value) return discretionarySpent.value > 0 ? 1 : 0
  return Math.min(discretionarySpent.value / freeMoney.value, 1)
})

// Bar grows from zero on reveal, in lockstep with the counting figures.
const envelopeBarWidth = computed(() => `${spentRatio.value * 100 * reveal.value}%`)

onMounted(() => run(async () => {
  await Promise.all([accountsStore.load(), store.loadMonths()])
  if (store.months.length > 0) {
    const m = store.months[0]
    selectedDate.value = DateTime.local(m.year, m.month, 1).startOf('month')
    await store.load(m.year, m.month)
    playReveal()
  }
}))
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Budget</h1>
      <MonthPicker :model-value="selectedDate" @update:model-value="onMonthChange" />
    </div>

    <!-- Load failure -->
    <PageError v-if="error" :message="error" @retry="retry" />

    <!-- Empty state: no months at all -->
    <p v-else-if="!store.months.length && !loading" class="text-muted text-sm">
      No transaction data yet.
    </p>

    <!-- Formula row + detail panel -->
    <template v-else-if="store.summary">
      <!-- Formula columns -->
      <div class="tmfi-rise flex border border-default rounded-lg">
        <!-- Income -->
        <button
          class="relative flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors flex-1 min-w-0 rounded-l-lg border-r border-default"
          :class="store.activeSection === 'income' ? 'bg-elevated' : ''"
          @click="setActiveSection('income')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Income</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.grossIncome * reveal) }}</span>
          <span class="text-xs text-muted">Net: {{ money(store.summary.netIncome * reveal) }}</span>
          <span class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 z-10 size-6 rounded-full border border-default flex items-center justify-center text-xs text-muted select-none pointer-events-none" style="background: var(--ui-bg)">−</span>
        </button>

        <!-- Savings -->
        <button
          class="relative flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors flex-1 min-w-0 border-r border-default"
          :class="store.activeSection === 'savings' ? 'bg-elevated' : ''"
          @click="setActiveSection('savings')"
        >
          <span class="flex items-center justify-between gap-2">
            <span class="text-xs font-semibold uppercase tracking-wide" :class="savingsMet ? 'text-primary' : 'text-muted'">Savings</span>
            <!-- Paid yourself first: target met or beaten this month -->
            <span
              v-if="savingsMet"
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
              Met
            </span>
          </span>
          <span class="text-xl font-bold tabular-nums" :class="savingsMet ? 'text-primary' : ''">{{ money(store.summary.savings.total * reveal) }}</span>

          <!-- Inline target edit -->
          <template v-if="editingTarget">
            <div class="flex items-center gap-1 mt-1" @click.stop>
              <CurrencyInput
                v-model="targetInput"
                size="xs"
                class="w-24"
                @keyup.enter="saveTarget"
                @keyup.escape="cancelTargetEdit"
              />
              <UButton size="xs" :loading="savingTarget" :disabled="savingTarget" @click="saveTarget">Save</UButton>
            </div>
          </template>
          <template v-else>
            <span class="text-xs flex items-center" :class="savingsMet ? 'text-primary' : 'text-muted'">
              <button
                class="inline-flex items-center gap-1 hover:opacity-100 transition-opacity group"
                title="Edit savings target"
                @click.stop="openTargetEdit"
              >
                {{ savingsMet ? 'target met' : savingsSubLabel }}
                <UIcon name="i-ph-pencil" class="w-3 h-3 opacity-50 group-hover:opacity-100 transition-opacity" />
              </button>
            </span>
          </template>
          <span class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 z-10 size-6 rounded-full border border-default flex items-center justify-center text-xs text-muted select-none pointer-events-none" style="background: var(--ui-bg)">−</span>
        </button>

        <!-- Taxes (non-clickable) -->
        <div class="relative flex flex-col gap-1 p-4 flex-1 min-w-0 border-r border-default">
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Taxes</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.taxes * reveal) }}</span>
          <span class="text-xs text-muted">withheld</span>
          <span class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 z-10 size-6 rounded-full border border-default flex items-center justify-center text-xs text-muted select-none pointer-events-none" style="background: var(--ui-bg)">−</span>
        </div>

        <!-- Fixed -->
        <button
          class="relative flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors flex-1 min-w-0 border-r border-default"
          :class="store.activeSection === 'fixed' ? 'bg-elevated' : ''"
          @click="setActiveSection('fixed')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Fixed</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.fixed.total * reveal) }}</span>
          <span class="text-xs text-muted">
            {{ store.summary.fixed.transactions.length }} transaction{{ store.summary.fixed.transactions.length === 1 ? '' : 's' }}
          </span>
          <span class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 z-10 size-6 rounded-full border border-default flex items-center justify-center text-xs text-muted select-none pointer-events-none" style="background: var(--ui-bg)">=</span>
        </button>

        <!-- Free Money (non-clickable, green bg) -->
        <div class="flex flex-col gap-1 p-4 border-r border-default bg-success/10 flex-1 min-w-0">
          <span class="text-xs font-semibold uppercase tracking-wide text-success">Free Money</span>
          <span class="text-xl font-bold tabular-nums text-success">{{ money(store.summary.freeMoney * reveal) }}</span>
          <span class="text-xs text-success/70">&nbsp;</span>
        </div>

        <!-- Discretionary -->
        <button
          class="flex flex-col gap-1 p-4 text-left hover:bg-elevated/60 transition-colors flex-1 min-w-0 rounded-r-lg"
          :class="store.activeSection === 'discretionary' ? 'bg-elevated' : ''"
          @click="setActiveSection('discretionary')"
        >
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Discretionary</span>
          <span class="text-xl font-bold tabular-nums">{{ money(store.summary.discretionary.total * reveal) }}</span>
          <span
            class="text-xs font-medium"
            :class="discretionaryRemaining >= 0 ? 'text-success' : 'text-error'"
          >
            {{ money(discretionaryRemaining * reveal) }} remaining
          </span>
        </button>
      </div>

      <!-- Free-money envelope: how much room is left to spend this month.
           The gauge fills with discretionary spending; the headroom that
           remains glows emerald until the line is crossed. -->
      <div v-if="showEnvelope" class="tmfi-rise" :style="{ animationDelay: '70ms' }">
        <div class="flex items-baseline justify-between gap-3 mb-2">
          <span class="text-xs font-semibold uppercase tracking-wide text-muted">Free Money Spent</span>
          <span v-if="overBudget" class="text-xs font-semibold font-mono tabular-nums text-error">
            {{ money(Math.abs(discretionaryRemaining) * reveal) }} over
          </span>
          <span v-else class="text-xs font-semibold font-mono tabular-nums text-success">
            {{ money(discretionaryRemaining * reveal) }} left
          </span>
        </div>
        <div
          role="progressbar"
          :aria-valuenow="Math.round(spentRatio * 100)"
          aria-valuemin="0"
          aria-valuemax="100"
          :aria-label="`Free money spent: ${money(discretionarySpent)} of ${money(freeMoney)}`"
          class="relative h-2.5 rounded-full bg-elevated overflow-hidden"
        >
          <div
            class="relative h-full rounded-full transition-[width] duration-500 ease-out"
            :class="overBudget ? 'bg-error' : 'bg-success'"
            :style="{ width: envelopeBarWidth }"
          />
        </div>
        <div class="mt-2 text-xs text-muted">
          <span class="font-mono tabular-nums font-medium text-default">{{ money(discretionarySpent * reveal) }}</span>
          spent of
          <span class="font-mono tabular-nums font-medium text-default">{{ money(freeMoney * reveal) }}</span>
          <template v-if="hasEnvelope"> free money this month</template>
          <template v-else> — no free money this month</template>
        </div>
      </div>

      <!-- Detail panel: the active section, always on top -->
      <div v-if="store.activeSection" class="tmfi-rise" :style="{ animationDelay: '120ms' }">
        <!-- Crossfade as the active section changes — the panel content swaps,
             so a quick fade conveys "this is now a different breakdown". -->
        <Transition
          enter-active-class="transition-opacity duration-150 ease-out"
          enter-from-class="opacity-0"
          leave-active-class="transition-opacity duration-100 ease-in"
          leave-to-class="opacity-0"
          mode="out-in"
        >
          <BudgetSectionCard
            :key="store.activeSection"
            :section="store.activeSection"
            :transactions="sectionTransactions(store.activeSection)"
            :month-start="monthStartIso"
            :carried-from-label="carriedFromLabel"
          />
        </Transition>
      </div>

      <!-- At-a-glance toggle: expand the remaining sections below -->
      <button
        class="flex items-center gap-1.5 text-xs text-muted font-medium rounded hover:text-default transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30"
        :aria-expanded="showAllSections"
        @click="showAllSections = !showAllSections"
      >
        <UIcon :name="showAllSections ? 'i-ph-caret-down' : 'i-ph-caret-right'" class="w-3.5 h-3.5" />
        <span>All sections</span>
      </button>

      <!-- The other sections, in formula order -->
      <template v-if="showAllSections">
        <BudgetSectionCard
          v-for="(section, i) in remainingSections"
          :key="section"
          class="tmfi-rise"
          :style="{ animationDelay: `${i * 50}ms` }"
          :section="section"
          :transactions="sectionTransactions(section)"
          :month-start="monthStartIso"
          :carried-from-label="carriedFromLabel"
        />
      </template>
    </template>

    <!-- Months known but summary not ready yet (loading) -->
    <div v-else class="flex items-center gap-2 text-muted text-sm py-10 justify-center">
      <UIcon name="i-ph-circle-notch" class="size-4 animate-spin" />
      Loading…
    </div>
  </div>
</template>
