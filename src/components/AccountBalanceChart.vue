<script setup lang="ts">
import { VisXYContainer, VisLine, VisAxis, VisCrosshair, VisTooltip } from '@unovis/vue'
import { DateTime } from 'luxon'

export type ChartPoint = { date: string; balance: number }

const props = defineProps<{ points: ChartPoint[]; mode: 'monthly' | 'intramonth' }>()

type D = { t: number; v: number }

const data = (): D[] => props.points.map(p => ({ t: DateTime.fromISO(p.date).toMillis(), v: p.balance }))
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
  <VisXYContainer :data="data()" :height="200">
    <VisLine :x="x" :y="y" />
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" />
    <VisCrosshair :x="x" :y="y" :template="crosshairTemplate" />
    <VisTooltip />
  </VisXYContainer>
</template>
