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

const crosshairTemplate = (d: D) => {
  const date = DateTime.fromMillis(d.t).toFormat('MMMM yyyy')
  const value = d.v.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
  return `<div style="padding:6px 10px;font-size:12px;line-height:1.6">
    <strong>${date}</strong><br/>Net Worth: ${value}
  </div>`
}
</script>

<template>
  <VisXYContainer :data="data()" :height="280">
    <VisLine :x="x" :y="y" />
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" />
    <VisCrosshair :x="x" :y="y" :template="crosshairTemplate" />
    <VisTooltip />
  </VisXYContainer>
</template>
