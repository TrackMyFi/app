<script setup lang="ts">
import { computed } from 'vue'
import { VisXYContainer, VisLine, VisArea, VisAxis, VisCrosshair, VisTooltip } from '@unovis/vue'
import { DateTime } from 'luxon'
import ZeroGradientDefs from './ZeroGradientDefs.vue'
import { useZeroThresholdGradient } from '../composables/useZeroThresholdGradient'

export type ChartPoint = { date: string; balance: number }

const props = defineProps<{ points: ChartPoint[]; mode: 'monthly' | 'intramonth' }>()

type D = { t: number; v: number }

// Memoized so the crosshair's per-mousemove re-render doesn't re-parse every date.
const data = computed<D[]>(() => props.points.map(p => ({ t: DateTime.fromISO(p.date).toMillis(), v: p.balance })))

const { paint, defs, pointColor } = useZeroThresholdGradient(() => data.value.map(d => d.v))
const x = (d: D) => d.t
const y = (d: D) => d.v

const tickFormatX = (t: number | Date) => {
  const ms = typeof t === 'number' ? t : t.getTime()
  return props.mode === 'monthly'
    ? DateTime.fromMillis(ms).toFormat('LLL yyyy')
    : DateTime.fromMillis(ms).toFormat('MMM d')
}

const crosshairTemplate = (d: D) => {
  const ms = d.t
  const date = props.mode === 'monthly'
    ? DateTime.fromMillis(ms).toFormat('MMMM yyyy')
    : DateTime.fromMillis(ms).toFormat('MMM d, yyyy')
  const balance = d.v.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
  return `<div style="padding:6px 10px;font-size:12px;line-height:1.6">
    <strong>${date}</strong><br/>Balance: ${balance}
  </div>`
}
</script>

<template>
  <ZeroGradientDefs v-bind="defs" />
  <VisXYContainer :data="data" :height="200">
    <VisArea :x="x" :y="y" :color="paint" :baseline="0" :opacity="0.1" />
    <VisLine :x="x" :y="y" :color="paint" :line-width="2" />
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" />
    <VisCrosshair :x="x" :y="y" :color="(d: D) => pointColor(d.v)" :template="crosshairTemplate" />
    <VisTooltip />
  </VisXYContainer>
</template>
