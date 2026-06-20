<script setup lang="ts">
import { VisXYContainer, VisLine, VisAxis, VisCrosshair, VisTooltip } from '@unovis/vue'
import type { ProjectionPoint } from '../lib/fire/projection'
import { DateTime } from 'luxon'

const props = defineProps<{
  points: ProjectionPoint[]
  fireNumber: number
  coastNumber: number
}>()

type D = { t: number; v: number; fire: number; coast: number }

const data = (): D[] => props.points.map(p => ({
  t: DateTime.fromISO(p.date).toMillis(),
  v: p.value,
  fire: props.fireNumber,
  coast: props.coastNumber,
}))

const x = (d: D) => d.t
const yValue = (d: D) => d.v
const yFire = (d: D) => d.fire
const yCoast = (d: D) => d.coast
const tickFormatX = (t: number | Date) =>
  DateTime.fromMillis(typeof t === 'number' ? t : t.getTime()).toFormat('yyyy')

function money(n: number) {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

const crosshairTemplate = (d: D) => {
  const date = DateTime.fromMillis(d.t).toFormat('yyyy')
  return `<div style="padding:6px 10px;font-size:12px;line-height:1.8">
    <strong>${date}</strong><br/>
    <span style="color:#6366f1">Portfolio: ${money(d.v)}</span><br/>
    <span style="color:#22c55e">FIRE Target: ${money(d.fire)}</span><br/>
    <span style="color:#f59e0b">Coast Target: ${money(d.coast)}</span>
  </div>`
}
</script>

<template>
  <VisXYContainer :data="data()" :height="280">
    <VisLine :x="x" :y="yValue" color="#6366f1" />
    <VisLine :x="x" :y="yFire" color="#22c55e" :line-dash-array="[4, 4]" />
    <VisLine :x="x" :y="yCoast" color="#f59e0b" :line-dash-array="[4, 4]" />
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" />
    <VisCrosshair :x="x" :y="yValue" :template="crosshairTemplate" />
    <VisTooltip />
  </VisXYContainer>
</template>
