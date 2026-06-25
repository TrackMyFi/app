<script setup lang="ts">
import { computed } from 'vue'
import { VisXYContainer, VisLine, VisArea, VisAxis, VisCrosshair, VisTooltip } from '@unovis/vue'
import type { NetWorthPoint } from '../lib/fire/netWorthSeries'
import { DateTime } from 'luxon'
import ZeroGradientDefs from './ZeroGradientDefs.vue'
import { useZeroThresholdGradient } from '../composables/useZeroThresholdGradient'

const props = defineProps<{ points: NetWorthPoint[] }>()

type D = { t: number; v: number }

// Memoized so the crosshair's per-mousemove re-render doesn't re-parse every date.
const data = computed<D[]>(() => props.points.map(p => ({ t: DateTime.fromISO(p.date).toMillis(), v: p.netWorth })))

const { paint, defs, pointColor } = useZeroThresholdGradient(() => data.value.map(d => d.v))
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
    <ZeroGradientDefs v-bind="defs" />
    <VisXYContainer :data="data" :height="280">
      <VisArea :x="x" :y="y" :color="paint" :baseline="0" :opacity="0.1" />
      <VisLine :x="x" :y="y" :color="paint" :line-width="2" />
      <VisAxis type="x" :tick-format="tickFormatX" />
      <VisAxis type="y" :tick-format="tickFormatY" />
      <VisCrosshair :x="x" :y="y" :color="(d: D) => pointColor(d.v)" :template="crosshairTemplate" />
      <VisTooltip />
    </VisXYContainer>
  </template>
  <div v-else class="h-[280px] flex items-center justify-center">
    <p class="text-sm text-muted">Add account balances to see your net worth over time.</p>
  </div>
</template>
