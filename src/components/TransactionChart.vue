<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { VisXYContainer, VisGroupedBar, VisLine, VisAxis, VisTooltip, VisCrosshair } from '@unovis/vue'
import { GroupedBar } from '@unovis/ts'
import { DateTime } from 'luxon'
import { isLiability } from '../lib/accountTypes'
import type { Transaction } from '../lib/types/Transaction'
import type { Account } from '../lib/types/Account'

const props = defineProps<{
  transactions: Transaction[]
  accounts: Account[]
}>()

// ─── Data aggregation ─────────────────────────────────────────────────────────

function effectiveDelta(t: Transaction): number {
  if (t.type === 'income') return t.amount
  if (t.type === 'expense') return -t.amount
  if (t.transferAccountId == null) return 0
  const destType = props.accounts.find(a => a.id === t.transferAccountId)?.type ?? ''
  return isLiability(destType) ? -t.amount : 0
}

type MonthPoint = { t: number; income: number; expense: number; net: number }

const monthlyAggregates = computed((): MonthPoint[] => {
  const byMonth = new Map<string, { income: number; expense: number; net: number }>()
  for (const t of props.transactions) {
    const month = t.date.slice(0, 7)
    if (!byMonth.has(month)) byMonth.set(month, { income: 0, expense: 0, net: 0 })
    const entry = byMonth.get(month)!
    const delta = effectiveDelta(t)
    if (delta > 0) entry.income += delta
    else if (delta < 0) entry.expense += Math.abs(delta)
    entry.net += delta
  }
  return [...byMonth.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([m, v]) => ({ t: DateTime.fromISO(m + '-01').toMillis(), ...v }))
})

// ─── Line chart: cumulative net ───────────────────────────────────────────────

type LinePoint = { t: number; v: number }

const lineData = computed((): LinePoint[] => {
  let running = 0
  return monthlyAggregates.value.map(m => {
    running += m.net
    return { t: m.t, v: running }
  })
})

const xLine = (d: LinePoint) => d.t
const yLine = (d: LinePoint) => d.v

// ─── Bar chart: income vs expense ─────────────────────────────────────────────

const xBar = (d: MonthPoint) => d.t
const yBar = [(d: MonthPoint) => d.income, (d: MonthPoint) => d.expense]

// Read semantic colors from the design system at mount time for Unovis SVG compatibility
const successColor = ref('#22c55e')
const errorColor = ref('#ef4444')

onMounted(() => {
  const el = document.createElement('span')
  document.body.appendChild(el)
  el.className = 'text-success'
  const s = getComputedStyle(el).color
  if (s) successColor.value = s
  el.className = 'text-error'
  const e = getComputedStyle(el).color
  if (e) errorColor.value = e
  document.body.removeChild(el)
})

const barColors = computed(() => [successColor.value, errorColor.value])

// ─── Formatters ───────────────────────────────────────────────────────────────

function money(n: number) {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

const tickFormatX = (t: number | Date) => {
  const ms = typeof t === 'number' ? t : t.getTime()
  return DateTime.fromMillis(ms).toFormat('LLL')
}

const tickFormatY = (v: number | Date) => {
  const n = typeof v === 'number' ? v : Number(v)
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

// ─── Line chart crosshair ─────────────────────────────────────────────────────

const lineCrosshairTemplate = (d: LinePoint) => {
  const month = DateTime.fromMillis(d.t).toFormat('MMMM yyyy')
  return `<div style="padding:6px 10px;font-size:12px;line-height:1.6">
    <strong>${month}</strong><br/>Cumulative Net: ${money(d.v)}
  </div>`
}

// ─── Bar chart tooltip ────────────────────────────────────────────────────────

const tooltipTriggers = computed(() => ({
  [GroupedBar.selectors.bar]: (d: MonthPoint) => {
    const month = DateTime.fromMillis(d.t).toFormat('MMMM yyyy')
    return `<div style="padding:6px 10px;font-size:12px;line-height:1.8">
      <strong>${month}</strong><br/>
      <span style="color:${successColor.value}">Income: ${money(d.income)}</span><br/>
      <span style="color:${errorColor.value}">Expense: ${money(d.expense)}</span><br/>
      Net: ${money(d.net)}
    </div>`
  },
}))
</script>

<template>
  <div class="grid grid-cols-3 gap-6">
    <!-- Cumulative net line: 2/3 -->
    <div class="col-span-2">
      <p class="text-xs font-medium text-muted mb-2">Cumulative Net</p>
      <VisXYContainer :data="lineData" :height="200">
        <VisLine :x="xLine" :y="yLine" />
        <VisAxis type="x" :tick-format="tickFormatX" />
        <VisAxis type="y" :tick-format="tickFormatY" />
        <VisCrosshair :x="xLine" :y="yLine" :template="lineCrosshairTemplate" />
        <VisTooltip />
      </VisXYContainer>
    </div>

    <!-- Income vs expense bars: 1/3 -->
    <div class="col-span-1">
      <p class="text-xs font-medium text-muted mb-2">Income vs. Expense</p>
      <VisXYContainer :data="monthlyAggregates" :height="200">
        <VisGroupedBar :x="xBar" :y="yBar" :color="barColors" />
        <VisAxis type="x" :tick-format="tickFormatX" />
        <VisTooltip :triggers="tooltipTriggers" />
      </VisXYContainer>
    </div>
  </div>
</template>
