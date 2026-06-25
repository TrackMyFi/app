<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { DateTime } from 'luxon'
import { useFireProfileStore } from '../stores/fireProfile'
import { useAccountsStore } from '../stores/accounts'
import { useContributionsStore } from '../stores/contributions'
import {
  activeFireInputs, investableNetWorth, savingsRate,
  derivedMonthlyContribution, buildForecast, projectionSeries, monthsToFire,
  realMonthlyReturn,
  type ForecastInputs, type VariantForecast, type FireVariant,
} from '../lib/fire'
import ForecastChart from '../components/ForecastChart.vue'
import { CHART_COLORS } from '../lib/forecastColors'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'
import { useReveal } from '../composables/useReveal'

const fp = useFireProfileStore()
const acc = useAccountsStore()
const contrib = useContributionsStore()
const { error, run, retry } = usePageData()

const open = ref(false)

onMounted(() => run(async () => {
  await Promise.all([fp.load(), acc.load(), contrib.load(DateTime.now().year)])
}))

const inputs = computed(() => activeFireInputs(acc.accounts, acc.allBalances))
const investable = computed(() => investableNetWorth(inputs.value.accounts, inputs.value.balances))
const asOf = computed(() => DateTime.now().toISODate()!)

const baseline = computed(() => {
  if (!fp.profile) return { monthly: 0, estimated: true }
  const rate = savingsRate(inputs.value.accounts, inputs.value.balances, fp.profile.annualIncome, asOf.value)
  const estimateMonthly = (fp.profile.annualIncome * rate) / 12
  return derivedMonthlyContribution(contrib.txns, asOf.value, estimateMonthly)
})

const ov = reactive<{ monthly: number | null; returnRate: number | null; inflation: number | null; retireAge: number | null }>({
  monthly: null, returnRate: null, inflation: null, retireAge: null,
})
const isScenario = computed(() =>
  ov.monthly !== null || ov.returnRate !== null || ov.inflation !== null || ov.retireAge !== null)

function reset() {
  ov.monthly = null; ov.returnRate = null; ov.inflation = null; ov.retireAge = null
}

const effMonthly = computed(() => ov.monthly ?? baseline.value.monthly)
const effReturn = computed(() => ov.returnRate ?? fp.profile?.expectedReturnRate ?? 0)
const effInflation = computed(() => ov.inflation ?? fp.profile?.inflationRate ?? 0)
const effRetireAge = computed(() => ov.retireAge ?? fp.profile?.targetRetirementAge ?? 0)

const forecastInputs = computed<ForecastInputs | null>(() => {
  if (!fp.profile) return null
  return {
    currentAge: fp.currentAge,
    targetRetirementAge: effRetireAge.value,
    annualExpensesTarget: fp.profile.annualExpensesTarget,
    leanFireAnnualExpenses: fp.profile.leanFireAnnualExpenses,
    fatFireAnnualExpenses: fp.profile.fatFireAnnualExpenses,
    expectedReturnRate: effReturn.value,
    inflationRate: effInflation.value,
    investable: investable.value,
    monthlyContribution: effMonthly.value,
  }
})

const forecasts = computed<VariantForecast[]>(() =>
  forecastInputs.value ? buildForecast(forecastInputs.value) : [])

const regular = computed(() => forecasts.value.find(f => f.variant === 'regular') ?? null)

// ── Baseline (no what-if overrides) ───────────────────────────────────────
// The forecast as it stands today, used to measure how far a scenario moves the
// FI date. Mirrors `forecastInputs` but ignores every override.
const baselineInputs = computed<ForecastInputs | null>(() => {
  if (!fp.profile) return null
  return {
    currentAge: fp.currentAge,
    targetRetirementAge: fp.profile.targetRetirementAge,
    annualExpensesTarget: fp.profile.annualExpensesTarget,
    leanFireAnnualExpenses: fp.profile.leanFireAnnualExpenses,
    fatFireAnnualExpenses: fp.profile.fatFireAnnualExpenses,
    expectedReturnRate: fp.profile.expectedReturnRate,
    inflationRate: fp.profile.inflationRate,
    investable: investable.value,
    monthlyContribution: baseline.value.monthly,
  }
})
const baselineRegular = computed(() =>
  baselineInputs.value ? buildForecast(baselineInputs.value).find(f => f.variant === 'regular') ?? null : null)

// ── What-if shift ─────────────────────────────────────────────────────────
// How a live scenario moves the FI date versus baseline — the page's feedback
// loop. "Sooner" is the motivating case (earn it emerald); "later" stays quiet.
type FiShift =
  | { kind: 'sooner' | 'later'; months: number }
  | { kind: 'reachable' | 'unreachable' }

const fiShift = computed<FiShift | null>(() => {
  if (!isScenario.value) return null
  const base = baselineRegular.value, scen = regular.value
  if (!base || !scen) return null
  const b = base.yearsToFi, s = scen.yearsToFi
  if (b === null && s === null) return null
  if (b === null) return { kind: 'reachable' }
  if (s === null) return { kind: 'unreachable' }
  const months = Math.round((b - s) * 12)
  if (months === 0) return null
  return { kind: months > 0 ? 'sooner' : 'later', months: Math.abs(months) }
})

function fmtDuration(months: number): string {
  const y = Math.floor(months / 12), m = months % 12
  if (y && m) return `${y} yr ${m} mo`
  if (y) return `${y} yr`
  return `${m} mo`
}

const shiftText = computed(() => {
  const s = fiShift.value
  if (!s) return ''
  if (s.kind === 'sooner') return `${fmtDuration(s.months)} sooner`
  if (s.kind === 'later') return `${fmtDuration(s.months)} later`
  if (s.kind === 'reachable') return 'Now within reach'
  return 'Out of reach'
})
const shiftClass = computed(() => {
  const s = fiShift.value
  if (!s) return ''
  if (s.kind === 'sooner' || s.kind === 'reachable') return 'bg-success/10 text-success'
  if (s.kind === 'unreachable') return 'bg-warning/10 text-warning'
  return 'bg-elevated text-muted'
})
const shiftIcon = computed(() => {
  const s = fiShift.value
  if (!s) return ''
  if (s.kind === 'sooner') return 'i-ph-fast-forward-fill'
  if (s.kind === 'reachable') return 'i-ph-check-circle-fill'
  if (s.kind === 'unreachable') return 'i-ph-warning-fill'
  return 'i-ph-hourglass-medium'
})

// The headline lands once: years-away and target count up while the date rises
// in. After the first reveal, scenario edits flow through instantly.
const { progress: reveal, play } = useReveal()
let revealed = false
watch(regular, r => { if (r && !revealed) { revealed = true; play() } }, { immediate: true })

const chartPoints = computed(() => {
  const fi = forecastInputs.value
  const reg = regular.value
  if (!fi || !reg) return []
  const mr = realMonthlyReturn(fi.expectedReturnRate, fi.inflationRate)
  const toFi = monthsToFire(fi.investable, fi.monthlyContribution, mr, reg.fireNumber)
  const toRet = Math.max(0, (fi.targetRetirementAge - fi.currentAge) * 12)
  const horizon = Math.min(1200, Math.round((toFi ?? toRet ?? 360) * 1.1) + 12)
  return projectionSeries(fi.investable, fi.monthlyContribution, fi.expectedReturnRate, fi.inflationRate, horizon)
})

const fmt = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
const labels: Record<FireVariant, string> = { lean: 'Lean FIRE', regular: 'FIRE', fat: 'Fat FIRE' }

const sMonthly = computed({ get: () => effMonthly.value, set: v => { ov.monthly = v ?? null } })
const sReturn = computed({ get: () => effReturn.value, set: v => { ov.returnRate = v ?? null } })
const sInflation = computed({ get: () => effInflation.value, set: v => { ov.inflation = v ?? null } })
const sRetire = computed({ get: () => effRetireAge.value, set: v => { ov.retireAge = v ?? null } })
</script>

<template>
  <div class="p-6 space-y-6">
    <PageError v-if="error" :message="error" @retry="retry" />

    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">Forecast</h1>
      <UButton v-if="fp.profile" icon="i-ph-sliders-horizontal" color="neutral" variant="outline" @click="open = true">
        What-if
      </UButton>
    </div>

    <div v-if="isScenario" class="flex items-center gap-3 text-sm rounded-lg border border-warning/40 bg-warning/10 px-3 py-2">
      <span class="text-warning font-medium">Scenario — not saved</span>
      <UButton size="xs" color="neutral" variant="ghost" @click="reset">Reset to baseline</UButton>
    </div>

    <!-- Empty state -->
    <div v-if="!fp.profile" class="border border-default rounded-lg p-10 text-center">
      <span class="i-ph-chart-line-up text-4xl text-muted block mx-auto mb-3" />
      <div class="font-semibold text-heading mb-1">No FIRE profile configured</div>
      <p class="text-sm text-muted mb-4">Set up your income, target expenses, and return assumptions to see projections.</p>
      <UButton to="/settings" color="neutral" variant="outline">Go to Settings</UButton>
    </div>

    <template v-else>
      <!-- Hero + chart -->
      <div v-if="regular" class="border border-default rounded-lg overflow-hidden">
        <div class="px-4 pt-4 pb-4 border-b border-default">
          <div class="text-xs font-semibold uppercase tracking-wider text-muted mb-1">{{ labels.regular }} date</div>
          <div class="text-3xl font-bold font-mono text-heading leading-none tmfi-rise">
            {{ regular.fiDate ? regular.fiDate.toFormat('LLL yyyy') : '—' }}
          </div>
          <div class="flex flex-wrap items-center gap-x-4 gap-y-2 mt-2 text-sm text-muted">
            <span v-if="regular.yearsToFi !== null" class="font-mono">{{ (regular.yearsToFi * reveal).toFixed(1) }}y away</span>
            <span class="font-mono">{{ fmt(regular.fireNumber * reveal) }} target</span>
            <Transition name="tmfi-fade">
              <span
                v-if="fiShift"
                :key="shiftText"
                class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium"
                :class="shiftClass"
              >
                <span :class="shiftIcon" class="size-3.5" />
                {{ shiftText }}
              </span>
            </Transition>
          </div>
        </div>
        <div class="p-4 pt-3">
          <ForecastChart
            :points="chartPoints"
            :fire-number="regular.fireNumber"
            :coast-number="regular.coastNumber"
            :crossing="regular.fiDate ? { date: regular.fiDate.toISODate()!, value: regular.fireNumber } : null"
          />
          <div class="flex gap-4 text-xs text-muted mt-2">
            <span>
              <span class="inline-block w-3 h-2 rounded-xs align-middle" :style="{ backgroundColor: CHART_COLORS.portfolio, opacity: 0.85 }" />
              Investable
            </span>
            <span>
              <span class="inline-block w-3 border-t-2 border-dashed align-middle" :style="{ borderColor: CHART_COLORS.fire }" />
              FIRE number
            </span>
            <span>
              <span class="inline-block w-3 border-t-2 border-dashed align-middle" :style="{ borderColor: CHART_COLORS.coast }" />
              Coast number
            </span>
          </div>
        </div>
      </div>

      <!-- Variant cards -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <UCard
          v-for="(v, i) in forecasts"
          :key="v.variant"
          class="tmfi-rise"
          :style="{ animationDelay: `${i * 70}ms` }"
          :class="v.variant === 'regular' ? 'ring-1 ring-primary' : ''"
        >
          <div class="text-xs font-semibold text-muted uppercase tracking-wider">{{ labels[v.variant] }}</div>
          <div class="text-2xl font-bold font-mono mt-1">{{ fmt(v.fireNumber) }}</div>
          <dl class="mt-3 space-y-1 text-sm">
            <div class="flex justify-between">
              <dt class="text-muted">Projected FI</dt>
              <dd>{{ v.fiDate ? v.fiDate.toFormat('LLL yyyy') : '—' }}</dd>
            </div>
            <div class="flex justify-between">
              <dt class="text-muted">Years to FI</dt>
              <dd class="font-mono">{{ v.yearsToFi !== null ? v.yearsToFi.toFixed(1) : '—' }}</dd>
            </div>
            <div class="flex justify-between">
              <dt class="text-muted">Coast number</dt>
              <dd class="font-mono">{{ fmt(v.coastNumber) }}</dd>
            </div>
            <div class="flex justify-between">
              <dt class="text-muted">Coast status</dt>
              <dd :class="v.coasting ? 'text-success' : ''">
                <template v-if="v.coasting">
                  <span class="i-ph-check-circle size-4 inline-block align-text-bottom mr-0.5" />Coasting
                </template>
                <template v-else>
                  {{ v.coastCrossingDate ? `by ${v.coastCrossingDate.toFormat('LLL yyyy')}` : '—' }}
                </template>
              </dd>
            </div>
            <div class="flex justify-between">
              <dt class="text-muted">Required monthly</dt>
              <dd class="font-mono">{{ v.requiredMonthly !== null ? fmt(v.requiredMonthly) : '—' }}</dd>
            </div>
          </dl>
        </UCard>
      </div>
    </template>

    <USlideover v-model:open="open" title="What-if scenario" side="right">
      <template #body>
        <div class="space-y-6">
          <div>
            <div class="flex justify-between items-center text-sm mb-2">
              <label class="text-muted">Monthly contribution</label>
              <input
                type="number"
                :value="Math.round(effMonthly)"
                @change="ov.monthly = Number(($event.target as HTMLInputElement).value)"
                :min="0" :max="20000" :step="100"
                class="w-24 text-right font-mono text-sm bg-transparent border border-default rounded px-2 py-0.5 focus:border-primary/50 focus:outline-none"
              />
            </div>
            <USlider v-model="sMonthly" :min="0" :max="20000" :step="100" />
            <p v-if="baseline.estimated" class="text-xs text-muted mt-1.5">
              Estimated — less than 12 months of contribution history.
            </p>
          </div>

          <div>
            <div class="flex justify-between items-center text-sm mb-2">
              <label class="text-muted">Expected return</label>
              <div class="flex items-center gap-0.5">
                <input
                  type="number"
                  :value="(sReturn * 100).toFixed(1)"
                  @change="ov.returnRate = Number(($event.target as HTMLInputElement).value) / 100"
                  :min="0" :max="15" :step="0.5"
                  class="w-16 text-right font-mono text-sm bg-transparent border border-default rounded px-2 py-0.5 focus:border-primary/50 focus:outline-none"
                />
                <span class="text-sm text-muted">%</span>
              </div>
            </div>
            <USlider v-model="sReturn" :min="0" :max="0.15" :step="0.005" />
          </div>

          <div>
            <div class="flex justify-between items-center text-sm mb-2">
              <label class="text-muted">Inflation</label>
              <div class="flex items-center gap-0.5">
                <input
                  type="number"
                  :value="(sInflation * 100).toFixed(1)"
                  @change="ov.inflation = Number(($event.target as HTMLInputElement).value) / 100"
                  :min="0" :max="10" :step="0.5"
                  class="w-16 text-right font-mono text-sm bg-transparent border border-default rounded px-2 py-0.5 focus:border-primary/50 focus:outline-none"
                />
                <span class="text-sm text-muted">%</span>
              </div>
            </div>
            <USlider v-model="sInflation" :min="0" :max="0.1" :step="0.005" />
          </div>

          <div>
            <div class="flex justify-between items-center text-sm mb-2">
              <label class="text-muted">Retirement age</label>
              <input
                type="number"
                :value="sRetire"
                @change="ov.retireAge = Number(($event.target as HTMLInputElement).value)"
                :min="fp.currentAge || 18" :max="80" :step="1"
                class="w-16 text-right font-mono text-sm bg-transparent border border-default rounded px-2 py-0.5 focus:border-primary/50 focus:outline-none"
              />
            </div>
            <USlider v-model="sRetire" :min="fp.currentAge || 18" :max="80" :step="1" />
          </div>

          <div v-if="isScenario" class="pt-2 border-t border-default">
            <UButton block color="neutral" variant="soft" @click="reset">Reset to baseline</UButton>
          </div>
        </div>
      </template>
    </USlideover>
  </div>
</template>
