<script setup lang="ts">
import { VisXYContainer, VisLine, VisAxis } from '@unovis/vue'
import type { NetWorthPoint } from '../lib/fire/netWorthSeries'
import { DateTime } from 'luxon'

const props = defineProps<{ points: NetWorthPoint[] }>()

type D = { t: number; v: number }

const data = (): D[] => props.points.map(p => ({ t: DateTime.fromISO(p.date).toMillis(), v: p.netWorth }))
const x = (d: D) => d.t
const y = (d: D) => d.v
const tickFormatX = (t: number | Date) =>
  DateTime.fromMillis(typeof t === 'number' ? t : t.getTime()).toFormat('LLL yyyy')
</script>

<template>
  <VisXYContainer :data="data()" :height="280">
    <VisLine :x="x" :y="y" />
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" />
  </VisXYContainer>
</template>
