<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { DateTime } from 'luxon'
import { useFireProfileStore } from '../stores/fireProfile'
import { useAccountsStore } from '../stores/accounts'
import {
  fireNumber, currentNetWorth, investableNetWorth, fiProgress,
  netWorthOverTime, projectedFiDate, savingsRate, activeFireInputs,
  derivedMonthlyContribution,
} from '../lib/fire'
import { useContributionsStore } from '../stores/contributions'
import StatCard from '../components/StatCard.vue'
import NetWorthChart from '../components/NetWorthChart.vue'

const fp = useFireProfileStore()
const acc = useAccountsStore()
const contrib = useContributionsStore()
onMounted(async () => {
  await Promise.all([fp.load(), acc.load(), contrib.load(DateTime.now().year)])
})

// Exclude archived (inactive) accounts and their balances from all metrics.
const inputs = computed(() => activeFireInputs(acc.accounts, acc.allBalances))
const fireAccounts = computed(() => inputs.value.accounts)
const fireBalances = computed(() => inputs.value.balances)

const fmt = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

const fireNum = computed(() => fp.profile ? fireNumber(fp.profile.annualExpensesTarget) : 0)
const netWorth = computed(() => currentNetWorth(fireAccounts.value, fireBalances.value))
const investable = computed(() => investableNetWorth(fireAccounts.value, fireBalances.value))
const progress = computed(() => fiProgress(investable.value, fireNum.value))
const series = computed(() => netWorthOverTime(fireAccounts.value, fireBalances.value))
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
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Contextual header -->
    <div>
      <h1 class="text-2xl font-bold text-balance">
        <template v-if="fiDate">Net worth: {{ fmt(netWorth) }} · FI in {{ yearsToFI }} year{{ yearsToFI === 1 ? '' : 's' }}</template>
        <template v-else-if="netWorth > 0">Net worth: {{ fmt(netWorth) }}</template>
        <template v-else>Your FIRE journey starts here</template>
      </h1>
      <p v-if="!fiDate && netWorth > 0" class="text-sm text-muted mt-1">
        Complete your FIRE profile in <router-link to="/settings" class="text-primary underline">Settings</router-link> to project your FI date.
      </p>
    </div>

    <!-- Hero metrics -->
    <div class="grid grid-cols-2 gap-4">
      <StatCard
        label="FI Progress"
        :value="`${progress.toFixed(1)}%`"
        :hero="true"
        :color="progress > 0 ? 'success' : undefined"
      />
      <StatCard label="Projected FI Date" :value="fiDate ? fiDate.toFormat('LLL yyyy') : '—'" :hero="true" />
    </div>

    <!-- Supporting metrics or first-run setup prompt -->
    <template v-if="acc.accounts.length === 0">
      <div class="rounded-lg border border-default bg-muted p-6 flex flex-col gap-4">
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
      <div class="grid grid-cols-2 lg:grid-cols-4 gap-3">
        <StatCard label="FIRE Number" :value="fmt(fireNum)" />
        <StatCard label="Current Net Worth" :value="fmt(netWorth)" />
        <StatCard label="Investable Net Worth" :value="fmt(investable)" />
        <StatCard
          label="Savings Rate"
          :value="`${(rate * 100).toFixed(1)}%`"
          :hint="contribution.estimated ? 'Estimated — under 12 months of contribution history' : undefined"
        />
      </div>
    </template>

    <!-- Net worth chart -->
    <div class="border border-default rounded-lg p-4">
      <h2 class="font-semibold mb-4">Net Worth Over Time</h2>
      <NetWorthChart :points="series" />
    </div>
  </div>
</template>
