<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
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

const fp = useFireProfileStore()
const acc = useAccountsStore()
const contrib = useContributionsStore()

const open = ref(false) // what-if drawer

onMounted(async () => {
  await Promise.all([fp.load(), acc.load(), contrib.load(DateTime.now().year)])
})

const inputs = computed(() => activeFireInputs(acc.accounts, acc.allBalances))
const investable = computed(() => investableNetWorth(inputs.value.accounts, inputs.value.balances))
const asOf = computed(() => DateTime.now().toISODate()!)

// Baseline derived monthly contribution (actual trailing-12mo, else savingsRate estimate).
const baseline = computed(() => {
  if (!fp.profile) return { monthly: 0, estimated: true }
  const rate = savingsRate(inputs.value.accounts, inputs.value.balances, fp.profile.annualIncome, asOf.value)
  const estimateMonthly = (fp.profile.annualIncome * rate) / 12
  return derivedMonthlyContribution(contrib.txns, asOf.value, estimateMonthly)
})

// What-if override state. null = use baseline/profile value.
const ov = reactive<{ monthly: number | null; returnRate: number | null; inflation: number | null; retireAge: number | null }>({
  monthly: null, returnRate: null, inflation: null, retireAge: null,
})
const isScenario = computed(() =>
  ov.monthly !== null || ov.returnRate !== null || ov.inflation !== null || ov.retireAge !== null)

function reset() {
  ov.monthly = null; ov.returnRate = null; ov.inflation = null; ov.retireAge = null
}

// Effective slider values (override ?? baseline/profile).
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

// Chart horizon: months to the Regular FI date, else months to retirement, else 30y. Padded.
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

function coastText(v: VariantForecast): string {
  if (v.coasting) return 'Coasting ✓'
  return v.coastCrossingDate ? `by ${v.coastCrossingDate.toFormat('LLL yyyy')}` : '—'
}

// Slider models bound to override values, seeded from effective values.
const sMonthly = computed({ get: () => effMonthly.value, set: v => { ov.monthly = v ?? null } })
const sReturn = computed({ get: () => effReturn.value, set: v => { ov.returnRate = v ?? null } })
const sInflation = computed({ get: () => effInflation.value, set: v => { ov.inflation = v ?? null } })
const sRetire = computed({ get: () => effRetireAge.value, set: v => { ov.retireAge = v ?? null } })
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">Forecast</h1>
      <UButton icon="i-ph-sliders-horizontal" color="neutral" variant="outline" @click="open = true">
        What-if
      </UButton>
    </div>

    <div v-if="isScenario" class="flex items-center gap-3 text-sm rounded-lg border border-warning/40 bg-warning/10 px-3 py-2">
      <span class="text-warning font-medium">Scenario — not saved</span>
      <UButton size="xs" color="neutral" variant="ghost" @click="reset">Reset to baseline</UButton>
    </div>

    <div v-if="regular" class="border border-default rounded-lg p-4">
      <h2 class="font-semibold mb-2">Projected growth — {{ labels.regular }}</h2>
      <ForecastChart :points="chartPoints" :fire-number="regular.fireNumber" :coast-number="regular.coastNumber" />
      <div class="flex gap-4 text-xs text-muted mt-2">
        <span><span class="inline-block w-3 border-t-2 align-middle" style="border-color:#6366f1" /> Investable</span>
        <span><span class="inline-block w-3 border-t-2 border-dashed align-middle" style="border-color:#22c55e" /> FIRE number</span>
        <span><span class="inline-block w-3 border-t-2 border-dashed align-middle" style="border-color:#f59e0b" /> Coast number</span>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <UCard v-for="v in forecasts" :key="v.variant" :class="v.variant === 'regular' ? 'ring-1 ring-primary' : ''">
        <div class="text-sm text-muted uppercase tracking-wide">{{ labels[v.variant] }}</div>
        <div class="text-xl font-semibold mt-1">{{ fmt(v.fireNumber) }}</div>
        <dl class="mt-3 space-y-1 text-sm">
          <div class="flex justify-between"><dt class="text-muted">Projected FI</dt>
            <dd>{{ v.fiDate ? v.fiDate.toFormat('LLL yyyy') : '—' }}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Years to FI</dt>
            <dd>{{ v.yearsToFi !== null ? v.yearsToFi.toFixed(1) : '—' }}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Coast number</dt>
            <dd>{{ fmt(v.coastNumber) }}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Coast status</dt>
            <dd :class="v.coasting ? 'text-success' : ''">{{ coastText(v) }}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Required / mo</dt>
            <dd>{{ v.requiredMonthly !== null ? fmt(v.requiredMonthly) : '—' }}</dd></div>
        </dl>
      </UCard>
    </div>

    <USlideover v-model:open="open" title="What-if scenario" side="right">
      <template #body>
        <div class="space-y-6">
          <div>
            <div class="flex justify-between text-sm mb-1">
              <label>Monthly contribution</label><span>{{ fmt(sMonthly) }}</span>
            </div>
            <USlider v-model="sMonthly" :min="0" :max="20000" :step="100" />
          </div>
          <div>
            <div class="flex justify-between text-sm mb-1">
              <label>Expected return</label><span>{{ (sReturn * 100).toFixed(1) }}%</span>
            </div>
            <USlider v-model="sReturn" :min="0" :max="0.15" :step="0.005" />
          </div>
          <div>
            <div class="flex justify-between text-sm mb-1">
              <label>Inflation</label><span>{{ (sInflation * 100).toFixed(1) }}%</span>
            </div>
            <USlider v-model="sInflation" :min="0" :max="0.1" :step="0.005" />
          </div>
          <div>
            <div class="flex justify-between text-sm mb-1">
              <label>Retirement age</label><span>{{ sRetire }}</span>
            </div>
            <USlider v-model="sRetire" :min="fp.currentAge || 18" :max="80" :step="1" />
          </div>
          <div v-if="isScenario" class="pt-2 border-t border-default">
            <UButton block color="neutral" variant="soft" @click="reset">Reset to baseline</UButton>
          </div>
          <p v-if="baseline.estimated" class="text-xs text-muted">
            Baseline contribution is estimated — less than 12 months of contribution history.
          </p>
        </div>
      </template>
    </USlideover>
  </div>
</template>
