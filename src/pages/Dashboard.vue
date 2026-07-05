<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { DateTime } from 'luxon'
import { useFireProfileStore } from '../stores/fireProfile'
import { useAccountsStore } from '../stores/accounts'
import {
  fireNumber, currentNetWorth, investableNetWorth, fiProgress,
  netWorthOverTime, investmentsOverTime, projectedFiDate, savingsRate, activeFireInputs,
  derivedMonthlyContribution, journeyProgress, portfolioMonthlyEarnings,
  coastStatus, safeMonthlyWithdrawal, crossoverStatus, buildMilestones,
  accessibleSplit, bridgeStatus, drawdownStatus, realMonthlyReturn,
} from '../lib/fire'
import { useContributionsStore } from '../stores/contributions'
import FiProgressCard from '../components/FiProgressCard.vue'
import NetWorthChart from '../components/NetWorthChart.vue'
import MilestoneLadder, { type MilestoneRow } from '../components/MilestoneLadder.vue'
import BridgeCard from '../components/BridgeCard.vue'
import InvestmentsChart from '../components/InvestmentsChart.vue'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'
import { useReveal } from '../composables/useReveal'

const fp = useFireProfileStore()
const acc = useAccountsStore()
const contrib = useContributionsStore()
const { error, run, retry } = usePageData()

// Drives the count-up reveal: the journey % and metrics tick into place once
// data lands, so the numbers feel earned rather than simply appearing.
const { progress: reveal, play: playReveal } = useReveal()

onMounted(() => run(async () => {
  await Promise.all([fp.load(), acc.load(), contrib.load(DateTime.now().year)])
  playReveal()
}))

// Exclude archived (inactive) accounts and their balances from all metrics.
const inputs = computed(() => activeFireInputs(acc.accounts, acc.allBalances))
const fireAccounts = computed(() => inputs.value.accounts)
const fireBalances = computed(() => inputs.value.balances)

const fmt = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
const fmtDelta = (n: number) => {
  const abs = Math.abs(n).toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
  return `${n >= 0 ? '+' : '−'}${abs}`
}

const swr = computed(() => fp.profile?.withdrawalRate ?? 0.04)
const swrPct = computed(() => `${+(swr.value * 100).toFixed(2)}%`)
const fireNum = computed(() => fp.profile ? fireNumber(fp.profile.annualExpensesTarget, swr.value) : 0)
const netWorth = computed(() => currentNetWorth(fireAccounts.value, fireBalances.value))
const investable = computed(() => investableNetWorth(fireAccounts.value, fireBalances.value))
const progress = computed(() => fiProgress(investable.value, fireNum.value))
const series = computed(() => netWorthOverTime(fireAccounts.value, fireBalances.value))
// Net worth with real-estate equity stripped out — null when no real estate is tracked.
const netWorthLessEquity = computed(() => series.value[series.value.length - 1]?.lessEquity ?? null)

// YTD change measures from the last recorded point before Jan 1 — the year-end
// snapshot the series carries forward. Null when history starts this year.
const ytdBaseline = computed(() => {
  const jan1 = DateTime.now().startOf('year').toISODate()!
  let base = null
  for (const p of series.value) {
    if (p.date >= jan1) break
    base = p
  }
  return base
})
const netWorthYtd = computed(() => ytdBaseline.value ? netWorth.value - ytdBaseline.value.netWorth : null)
const lessEquityYtd = computed(() =>
  ytdBaseline.value?.lessEquity != null && netWorthLessEquity.value != null
    ? netWorthLessEquity.value - ytdBaseline.value.lessEquity
    : null)
const hasLiquidAccounts = computed(() => fireAccounts.value.some(a => ['checking', 'savings'].includes(a.type)))

const investmentSeries = computed(() => investmentsOverTime(fireAccounts.value, fireBalances.value))
const investmentAccounts = computed(() =>
  investmentSeries.value.accountIds.map(id => ({
    id,
    name: acc.accounts.find(a => a.id === id)?.name ?? `Account ${id}`,
  }))
)

const asOf = computed(() => DateTime.now().toISODate()!)
const contribution = computed(() => {
  if (!fp.profile) return { monthly: 0, estimated: true }
  const estRate = savingsRate(fireAccounts.value, fireBalances.value, fp.profile.annualIncome, asOf.value)
  const estimateMonthly = (fp.profile.annualIncome * estRate) / 12
  return derivedMonthlyContribution(contrib.txns, asOf.value, estimateMonthly)
})
const rate = computed(() => fp.profile && fp.profile.annualIncome > 0
  ? (contribution.value.monthly * 12) / fp.profile.annualIncome
  : 0)
const fiDate = computed(() => fp.profile
  ? projectedFiDate(investable.value, contribution.value.monthly, fp.profile.expectedReturnRate, fp.profile.inflationRate, fireNum.value)
  : null)

const yearsToFI = computed(() => {
  if (!fiDate.value) return null
  return Math.round(fiDate.value.diff(DateTime.now(), 'years').years)
})

const journeyProg = computed(() => {
  if (!fp.profile) return null
  return journeyProgress(investable.value, contribution.value.monthly, fp.profile.expectedReturnRate, fp.profile.inflationRate, fireNum.value)
})

const portfolioEarnings = computed(() =>
  fp.profile ? portfolioMonthlyEarnings(investable.value, fp.profile.expectedReturnRate) : 0
)

// The same growth restated in today's purchasing power — the honest number.
const portfolioEarnsHint = computed(() => {
  if (!fp.profile || fp.profile.inflationRate <= 0) return 'Nominal monthly growth from compounding alone'
  const real = investable.value * realMonthlyReturn(fp.profile.expectedReturnRate, fp.profile.inflationRate)
  return `Nominal · ≈ ${fmt(real)} in today's dollars`
})

// All-time-high context under the header: quiet when climbing, honest in a dip.
const drawdown = computed(() => {
  if (series.value.length < 2) return null
  return drawdownStatus(series.value)
})

const coast = computed(() => {
  if (!fp.profile || fp.currentAge === 0) return null
  return coastStatus(
    investable.value,
    contribution.value.monthly,
    fireNum.value,
    fp.currentAge,
    fp.profile.targetRetirementAge,
    fp.profile.expectedReturnRate,
    fp.profile.inflationRate,
  )
})

// MoM delta — only shown when every investable account has a balance record older than
// one month. New accounts added this month would otherwise inflate the delta.
const oneMonthAgoIso = computed(() => DateTime.now().minus({ months: 1 }).toISODate()!)

const moMDeltas = computed(() => {
  const cutoff = oneMonthAgoIso.value
  const priorBalances = fireBalances.value.filter(b => b.recordedAt <= cutoff)
  if (priorBalances.length === 0) return null
  const accountsWithPrior = new Set(priorBalances.map(b => b.accountId))
  const investableAccounts = fireAccounts.value.filter(a => a.includeInFireCalculations)
  if (!investableAccounts.every(a => accountsWithPrior.has(a.id))) return null
  return {
    netWorth: netWorth.value - currentNetWorth(fireAccounts.value, priorBalances),
    investable: investable.value - investableNetWorth(fireAccounts.value, priorBalances),
  }
})

const investableTrend = computed(() => {
  if (!moMDeltas.value) return null
  const d = moMDeltas.value.investable
  return { text: `${fmtDelta(d)} this month`, positive: d >= 0 }
})

// Static secondary-info hints for cards that need a second line to equalize row heights.
const fireNumberHint = computed(() => {
  if (!fp.profile) return undefined
  const mult = 1 / swr.value
  return `${fmt(fp.profile.annualExpensesTarget)} annual expenses × ${Number.isInteger(mult) ? mult : mult.toFixed(1)}`
})

const contributionHint = computed(() => {
  const annual = fmt(contribution.value.monthly * 12)
  return contribution.value.estimated ? `Estimated · ${annual} / yr` : `${annual} / yr`
})

const savingsRateHint = computed(() => {
  if (!fp.profile || fp.profile.annualIncome === 0) return undefined
  return `of ${fmt(fp.profile.annualIncome)} annual income`
})

// Retirement age comparison: how many years before/after target retirement the FI date lands.
const retirementYearsAhead = computed(() => {
  if (!fiDate.value || !fp.profile || fp.currentAge === 0) return null
  const yearsToRetire = fp.profile.targetRetirementAge - fp.currentAge
  if (yearsToRetire <= 0) return null
  const retireDate = DateTime.now().plus({ years: yearsToRetire })
  return Math.round(retireDate.diff(fiDate.value, 'years').years)
})

// The portfolio as an employee: what it could sustainably pay out today.
const portfolioPays = computed(() => safeMonthlyWithdrawal(investable.value, swr.value))
const portfolioPaysHint = computed(() => {
  if (!fp.profile || fp.profile.annualExpensesTarget === 0) return `Sustainable income at a ${swrPct.value} withdrawal rate`
  return `${swrPct.value} rule · covers ${Math.min(progress.value, 100).toFixed(0)}% of your ${fmt(fp.profile.annualExpensesTarget / 12)}/mo expenses`
})

// Crossover point: when compounding out-earns the monthly contribution.
const crossover = computed(() => fp.profile
  ? crossoverStatus(investable.value, contribution.value.monthly, fp.profile.expectedReturnRate)
  : null)
const crossoverValue = computed(() => {
  if (!crossover.value) return '—'
  if (crossover.value.crossed) return 'Crossed'
  return crossover.value.date ? crossover.value.date.toFormat('LLL yyyy') : '—'
})
const crossoverHint = computed(() => {
  if (crossover.value?.crossed) return 'Your portfolio now out-earns your monthly contributions'
  if (crossover.value?.date) return `When your portfolio out-earns your ${fmt(contribution.value.monthly)}/mo contributions`
  return 'When your portfolio out-earns your contributions'
})

const yearsAway = (d: DateTime) => {
  const yrs = Math.round(d.diff(DateTime.now(), 'years').years)
  return yrs < 1 ? 'under a year' : `${yrs} yr${yrs === 1 ? '' : 's'}`
}

const milestones = computed<MilestoneRow[]>(() => {
  if (!fp.profile) return []
  const ladder = buildMilestones({
    investable: investable.value,
    monthlyContribution: contribution.value.monthly,
    expectedReturnRate: fp.profile.expectedReturnRate,
    inflationRate: fp.profile.inflationRate,
    annualExpensesTarget: fp.profile.annualExpensesTarget,
    leanFireAnnualExpenses: fp.profile.leanFireAnnualExpenses,
    fatFireAnnualExpenses: fp.profile.fatFireAnnualExpenses,
    coastNumber: coast.value?.coastNumber ?? null,
    withdrawalRate: swr.value,
  })
  const nextKey = ladder.find(m => !m.achieved)?.key
  return ladder.map(m => ({
    key: m.key,
    label: m.label,
    targetLabel: fmt(m.target),
    achieved: m.achieved,
    next: m.key === nextKey,
    dateLabel: m.projectedDate ? `${m.projectedDate.toFormat('LLL yyyy')} · ${yearsAway(m.projectedDate)}` : null,
  }))
})

// Bridge to 59½: an early FI date must be funded from accessible accounts.
const split = computed(() => accessibleSplit(fireAccounts.value, fireBalances.value))
const bridgePct = computed(() => {
  const acc = Math.max(0, split.value.accessible)
  const total = acc + Math.max(0, split.value.deferred)
  return total > 0 ? (acc / total) * 100 : 0
})
const bridge = computed(() => {
  if (!fp.profile || fp.currentAge === 0) return null
  const yearsExact = progress.value >= 100 ? 0
    : fiDate.value ? fiDate.value.diff(DateTime.now(), 'years').years : null
  if (yearsExact === null) return null
  return bridgeStatus(
    split.value, fp.currentAge, yearsExact, fp.profile.annualExpensesTarget,
    fp.profile.expectedReturnRate, fp.profile.inflationRate,
  )
})
const BRIDGE_CAVEAT = 'Projected from today\'s accessible balance alone — future contributions to taxable accounts aren\'t counted.'
const bridgeDisplay = computed<{ text: string; color: 'success' | 'warning' | 'muted'; caveat?: string }>(() => {
  if (!fp.profile || fp.currentAge === 0) {
    return { text: 'Add your date of birth in Settings to assess your bridge.', color: 'muted' }
  }
  const b = bridge.value
  if (!b) return { text: 'The bridge is assessed once a projected FI date is available.', color: 'muted' }
  if (!b.needed) {
    return { text: `You reach FI at age ${Math.round(b.ageAtFi)} — past 59½, so no bridge is needed.`, color: 'success' }
  }
  const yrs = Math.round(b.bridgeYears)
  const summary = `${fmt(b.bridgeNeeded)} carries you the ${yrs} year${yrs === 1 ? '' : 's'} from FI (age ${Math.round(b.ageAtFi)}) to 59½`
  if ((b.coverage ?? 0) >= 1) {
    return { text: `On track: ~${fmt(b.projectedAccessibleAtFi)} accessible at FI. ${summary}.`, color: 'success', caveat: BRIDGE_CAVEAT }
  }
  return {
    text: `Thin bridge: ~${fmt(b.projectedAccessibleAtFi)} accessible at FI covers ${((b.coverage ?? 0) * 100).toFixed(0)}% of it. ${summary}.`,
    color: 'warning',
    caveat: BRIDGE_CAVEAT,
  }
})

const coastHint = computed(() => {
  if (!coast.value) return 'Add your date of birth in Settings'
  if (coast.value.coasting) return 'Portfolio compounds to your goal without contributions'
  const d = coast.value.crossingDate
  if (!d) return 'Not reachable at current contribution rate'
  const yrs = Math.round(d.diff(DateTime.now(), 'years').years)
  return `coast by ${d.toFormat('LLL yyyy')} · ${yrs} yr${yrs === 1 ? '' : 's'}`
})

// The eight supporting metrics, consolidated into one card under the net worth
// headline. Row one is targets and waypoints; row two is the monthly flows.
interface Metric {
  label: string
  value: string
  hint?: string
  color?: 'success' | 'default'
  trend?: { text: string; positive: boolean } | null
}

const metrics = computed<Metric[]>(() => [
  { label: 'FIRE Number', value: fmt(fireNum.value * reveal.value), hint: fireNumberHint.value },
  { label: 'Investable Net Worth', value: fmt(investable.value * reveal.value), trend: investableTrend.value },
  {
    label: 'Coast FIRE',
    value: !coast.value ? '—' : coast.value.coasting ? 'Coasting' : fmt(coast.value.coastNumber * reveal.value),
    color: coast.value?.coasting ? 'success' : 'default',
    hint: coastHint.value,
  },
  {
    label: 'Crossover Point',
    value: crossoverValue.value,
    color: crossover.value?.crossed ? 'success' : 'default',
    hint: crossoverHint.value,
  },
  { label: 'Monthly Contribution', value: fmt(contribution.value.monthly * reveal.value), hint: contributionHint.value },
  { label: 'Savings Rate', value: `${(rate.value * 100 * reveal.value).toFixed(1)}%`, hint: savingsRateHint.value },
  { label: 'Portfolio Earns / Mo', value: fmt(portfolioEarnings.value * reveal.value), hint: portfolioEarnsHint.value },
  { label: 'Portfolio Pays / Mo', value: fmt(portfolioPays.value * reveal.value), hint: portfolioPaysHint.value },
])
</script>

<template>
  <div class="p-6 space-y-6">
    <PageError v-if="error" :message="error" @retry="retry" />

    <!-- First-run greeting; once accounts exist, net worth heads the metrics card below -->
    <div v-if="acc.accounts.length === 0">
      <h1 class="text-2xl font-bold text-balance">Your FIRE journey starts here</h1>
    </div>
    <p v-if="!fiDate && netWorth > 0" class="text-sm text-muted">
      Complete your FIRE profile in <router-link to="/settings/profile" class="text-primary underline">Settings</router-link> to project your FI date.
    </p>

    <!-- The journey: the long game made tangible -->
    <FiProgressCard
      :progress="progress"
      :reveal="reveal"
      :investable-label="fmt(investable)"
      :goal-label="fmt(fireNum)"
      :fi-date-label="fiDate ? fiDate.toFormat('LLL yyyy') : undefined"
      :years-to-fi="yearsToFI"
      :journey-progress="journeyProg"
      :target-retirement-age="fp.profile?.targetRetirementAge ?? null"
      :retirement-years-ahead="retirementYearsAhead"
    />

    <!-- Supporting metrics or first-run setup prompt -->
    <template v-if="acc.accounts.length === 0">
      <div class="tmfi-rise rounded-lg border border-default bg-muted p-6 flex flex-col gap-4">
        <UIcon name="i-ph-chart-line-up" class="w-6 h-6 text-success" />
        <div>
          <p class="text-sm font-semibold">Add accounts to unlock your metrics</p>
          <p class="text-sm text-muted mt-1">Your FI Progress, net worth, and projected independence date calculate automatically from your account balances.</p>
        </div>
        <div>
          <UButton to="/accounts" color="primary" size="sm" trailing-icon="i-ph-arrow-right">
            Add your first account
          </UButton>
        </div>
      </div>
    </template>
    <template v-else>
      <!-- Where you stand: net worth headline over its eight supporting metrics -->
      <section class="tmfi-rise border border-default rounded-lg" :style="{ animationDelay: '40ms' }">
        <header class="px-4 sm:px-5 pt-4 pb-4 flex flex-wrap items-start gap-x-10 gap-y-3">
          <div>
            <h2 class="text-sm text-muted">Net Worth</h2>
            <div class="mt-0.5 font-mono font-bold tabular-nums text-3xl text-highlighted">{{ fmt(netWorth * reveal) }}</div>
            <div
              v-if="moMDeltas"
              class="mt-0.5 text-xs font-mono tabular-nums"
              :class="moMDeltas.netWorth >= 0 ? 'text-success' : 'text-error'"
            >
              {{ fmtDelta(moMDeltas.netWorth) }} this month
            </div>
            <div
              v-if="netWorthYtd != null"
              class="mt-0.5 text-xs font-mono tabular-nums"
              :class="netWorthYtd >= 0 ? 'text-success' : 'text-error'"
            >
              {{ fmtDelta(netWorthYtd) }} YTD
            </div>
          </div>
          <div v-if="netWorthLessEquity != null">
            <div class="text-sm text-muted">Excl. Home Equity</div>
            <div class="mt-0.5 font-mono font-semibold tabular-nums text-3xl text-highlighted">{{ fmt(netWorthLessEquity * reveal) }}</div>
            <div
              v-if="lessEquityYtd != null"
              class="mt-0.5 text-xs font-mono tabular-nums"
              :class="lessEquityYtd >= 0 ? 'text-success' : 'text-error'"
            >
              {{ fmtDelta(lessEquityYtd) }} YTD
            </div>
          </div>
          <span v-if="drawdown" class="ms-auto text-xs" :class="drawdown.atHigh ? 'text-success' : 'text-muted'">
            <template v-if="drawdown.atHigh">At an all-time high</template>
            <template v-else>
              −{{ (drawdown.drawdown * 100).toFixed(1) }}% from the {{ DateTime.fromISO(drawdown.highDate).toFormat('LLL yyyy') }} high of {{ fmt(drawdown.high) }}
            </template>
          </span>
        </header>
        <dl class="border-t border-default px-4 sm:px-5 py-4 grid grid-cols-2 lg:grid-cols-4 gap-x-6 gap-y-5">
          <div v-for="(m, i) in metrics" :key="m.label" class="tmfi-rise" :style="{ animationDelay: `${80 + i * 40}ms` }">
            <dt class="text-sm text-muted">{{ m.label }}</dt>
            <dd
              class="mt-0.5 font-mono font-semibold tabular-nums text-xl"
              :class="m.color === 'success' ? 'text-success' : 'text-highlighted'"
            >
              {{ m.value }}
            </dd>
            <dd v-if="m.hint" class="mt-0.5 text-xs text-muted">{{ m.hint }}</dd>
            <dd
              v-if="m.trend"
              class="mt-0.5 text-xs font-mono tabular-nums"
              :class="m.trend.positive ? 'text-success' : 'text-error'"
            >
              {{ m.trend.text }}
            </dd>
          </div>
        </dl>
      </section>

    </template>

    <!-- Net worth chart, with the milestone ladder alongside setting the row height -->
    <div class="grid gap-3 items-stretch" :class="milestones.length > 0 ? 'xl:grid-cols-[minmax(0,1fr)_minmax(0,32rem)]' : ''">
      <div class="tmfi-rise border border-default rounded-lg p-4 flex flex-col" :style="{ animationDelay: '440ms' }">
        <h2 class="font-semibold mb-4">Net Worth Over Time</h2>
        <NetWorthChart :points="series" :show-liquid-series="hasLiquidAccounts" :fluid="milestones.length > 0" />
      </div>
      <div v-if="milestones.length > 0" class="tmfi-rise" :style="{ animationDelay: '480ms' }">
        <MilestoneLadder :milestones="milestones" class="h-full" />
      </div>
    </div>

    <!-- Investments chart -->
    <div v-if="investmentSeries.points.length > 0" class="tmfi-rise border border-default rounded-lg p-4" :style="{ animationDelay: '520ms' }">
      <h2 class="font-semibold mb-4">Investments Over Time</h2>
      <InvestmentsChart :points="investmentSeries.points" :accounts="investmentAccounts" />
    </div>

    <!-- The pre-59½ funding gap every early retiree has to plan around -->
    <div v-if="acc.accounts.length > 0" class="tmfi-rise" :style="{ animationDelay: '560ms' }">
      <BridgeCard
        :accessible-label="fmt(split.accessible)"
        :deferred-label="fmt(split.deferred)"
        :accessible-pct="bridgePct"
        :status-text="bridgeDisplay.text"
        :status-color="bridgeDisplay.color"
        :caveat="bridgeDisplay.caveat"
      />
    </div>
  </div>
</template>
