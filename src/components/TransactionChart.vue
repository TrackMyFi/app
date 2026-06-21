<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { VisXYContainer, VisLine, VisAxis, VisTooltip, VisCrosshair } from '@unovis/vue'
import { DateTime } from 'luxon'
import { classifyFlow } from '../lib/transactions/flow'
import type { Transaction } from '../lib/types/Transaction'
import type { Account } from '../lib/types/Account'

const props = defineProps<{
  transactions: Transaction[]
  accounts: Account[]
}>()

// ─── Data aggregation ─────────────────────────────────────────────────────────

type MonthPoint = { t: number; income: number; expense: number; net: number }

const monthlyAggregates = computed((): MonthPoint[] => {
  const byMonth = new Map<string, { income: number; expense: number; net: number }>()
  for (const t of props.transactions) {
    const month = t.date.slice(0, 7)
    if (!byMonth.has(month)) byMonth.set(month, { income: 0, expense: 0, net: 0 })
    const entry = byMonth.get(month)!
    const f = classifyFlow(t, props.accounts)
    entry.income += f.inflow
    if (!f.isSavings) entry.expense += f.outflow
    entry.net += f.inflow - (f.isSavings ? 0 : f.outflow)
  }
  return [...byMonth.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([m, v]) => ({ t: DateTime.fromISO(m + '-01').toMillis(), ...v }))
})

// ─── Cumulative net line ───────────────────────────────────────────────────────

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

// ─── Formatters ───────────────────────────────────────────────────────────────

function money(n: number) {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

const tickFormatX = (t: number | Date) => {
  const ms = typeof t === 'number' ? t : t.getTime()
  return DateTime.fromMillis(ms).toFormat('LLL yy')
}

const tickFormatY = (v: number | Date) => {
  const n = typeof v === 'number' ? v : Number(v)
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

// ─── Crosshair ────────────────────────────────────────────────────────────────

const lineCrosshairTemplate = (d: LinePoint) => {
  const month = DateTime.fromMillis(d.t).toFormat('MMMM yyyy')
  return `<div style="padding:6px 10px;font-size:12px;line-height:1.6">
    <strong>${month}</strong><br/>Cumulative Net: ${money(d.v)}
  </div>`
}

// Read semantic colors from the design system at mount time for Unovis SVG compatibility
const primaryColor = ref('#10b981')

onMounted(() => {
  const el = document.createElement('span')
  document.body.appendChild(el)
  el.className = 'text-primary'
  const s = getComputedStyle(el).color
  if (s) primaryColor.value = s
  document.body.removeChild(el)
})
</script>

<template>
  <VisXYContainer :data="lineData" :height="220">
    <VisLine :x="xLine" :y="yLine" :color="primaryColor" />
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" :tick-format="tickFormatY" />
    <VisCrosshair :x="xLine" :y="yLine" :template="lineCrosshairTemplate" />
    <VisTooltip />
  </VisXYContainer>
</template>
