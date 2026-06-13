<script setup lang="ts">
import { VisXYContainer, VisLine, VisAxis } from '@unovis/vue'
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
</script>

<template>
  <VisXYContainer :data="data()" :height="280">
    <VisLine :x="x" :y="yValue" color="#6366f1" />
    <VisLine :x="x" :y="yFire" color="#22c55e" :line-dash-array="[4, 4]" />
    <VisLine :x="x" :y="yCoast" color="#f59e0b" :line-dash-array="[4, 4]" />
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" />
  </VisXYContainer>
</template>
