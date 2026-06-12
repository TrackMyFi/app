<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { DateTime } from 'luxon'
import { useFireProfileStore } from '../stores/fireProfile'
import { useAccountsStore } from '../stores/accounts'
import {
  fireNumber, currentNetWorth, investableNetWorth, fiProgress,
  netWorthOverTime, projectedFiDate, savingsRate,
} from '../lib/fire'
import type { FireAccount, FireBalance } from '../lib/fire/types'
import StatCard from '../components/StatCard.vue'
import NetWorthChart from '../components/NetWorthChart.vue'

const fp = useFireProfileStore()
const acc = useAccountsStore()
onMounted(async () => { await Promise.all([fp.load(), acc.load()]) })

const fireAccounts = computed<FireAccount[]>(() =>
  acc.accounts.map(a => ({ id: a.id, type: a.type, includeInFireCalculations: a.includeInFireCalculations })))
const fireBalances = computed<FireBalance[]>(() =>
  acc.allBalances.map(b => ({ accountId: b.accountId, balance: b.balance, recordedAt: b.recordedAt })))

const fmt = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

const fireNum = computed(() => fp.profile ? fireNumber(fp.profile.annualExpensesTarget) : 0)
const netWorth = computed(() => currentNetWorth(fireAccounts.value, fireBalances.value))
const investable = computed(() => investableNetWorth(fireAccounts.value, fireBalances.value))
const progress = computed(() => fiProgress(investable.value, fireNum.value))
const series = computed(() => netWorthOverTime(fireAccounts.value, fireBalances.value))
const rate = computed(() => fp.profile
  ? savingsRate(fireAccounts.value, fireBalances.value, fp.profile.annualIncome, DateTime.now().toISODate()!)
  : 0)
const fiDate = computed(() => {
  if (!fp.profile) return null
  const monthly = (fp.profile.annualIncome * rate.value) / 12
  return projectedFiDate(investable.value, monthly, fp.profile.expectedReturnRate, fp.profile.inflationRate, fireNum.value)
})
</script>

<template>
  <div class="p-6 space-y-6">
    <h1 class="text-2xl font-bold">Dashboard</h1>
    <div class="grid grid-cols-2 lg:grid-cols-3 gap-4">
      <StatCard label="FIRE Number" :value="fmt(fireNum)" />
      <StatCard label="Current Net Worth" :value="fmt(netWorth)" />
      <StatCard label="Investable Net Worth" :value="fmt(investable)" />
      <StatCard label="FI Progress" :value="`${progress.toFixed(1)}%`" />
      <StatCard label="Projected FI Date" :value="fiDate ? fiDate.toFormat('LLL yyyy') : '—'" />
      <StatCard label="Savings Rate" :value="`${(rate * 100).toFixed(1)}%`" hint="Approximate — refined in Phase 2" />
    </div>
    <div class="border border-default rounded-lg p-4">
      <h2 class="font-semibold mb-2">Net Worth Over Time</h2>
      <NetWorthChart :points="series" />
    </div>
  </div>
</template>
