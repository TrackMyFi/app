<script setup lang="ts">
import { VisXYContainer, VisLine, VisAxis, VisCrosshair, VisTooltip } from '@unovis/vue'
import type { NetWorthPoint } from '../lib/fire/netWorthSeries'
import { DateTime } from 'luxon'

const props = defineProps<{ points: NetWorthPoint[] }>()

type D = { t: number; v: number }

const data = (): D[] => props.points.map(p => ({ t: DateTime.fromISO(p.date).toMillis(), v: p.netWorth }))
const x = (d: D) => d.t
const y = (d: D) => d.v
const tickFormatX = (t: number | Date) =>
  DateTime.fromMillis(typeof t === 'number' ? t : t.getTime()).toFormat('LLL yyyy')

const tickFormatY = (v: number | Date) => {
  const n = typeof v === 'number' ? v : 0
  if (Math.abs(n) >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`
  if (Math.abs(n) >= 1_000) return `$${(n / 1_000).toFixed(0)}k`
  return `$${n.toFixed(0)}`
}

const crosshairTemplate = (d: D) => {
  const date = DateTime.fromMillis(d.t).toFormat('MMMM yyyy')
  const value = d.v.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
  return `<div style="padding:6px 10px;font-size:12px;line-height:1.6">
    <strong>${date}</strong><br/>Net Worth: ${value}
  </div>`
}
</script>

<template>
  <template v-if="points.length > 0">
    <VisXYContainer :data="data()" :height="280">
      <VisLine :x="x" :y="y" />
      <VisAxis type="x" :tick-format="tickFormatX" />
      <VisAxis type="y" :tick-format="tickFormatY" />
      <VisCrosshair :x="x" :y="y" :template="crosshairTemplate" />
      <VisTooltip />
    </VisXYContainer>
  </template>
  <div v-else class="h-[280px] flex items-center justify-center">
    <p class="text-sm text-muted">Add account balances to see your net worth over time.</p>
  </div>
</template>
