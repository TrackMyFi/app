<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { VisXYContainer, VisLine, VisArea, VisAxis, VisCrosshair, VisTooltip, VisBulletLegend } from '@unovis/vue'
import type { InvestmentPoint } from '../lib/fire/investmentSeries'
import { DateTime } from 'luxon'

const props = defineProps<{
  points: InvestmentPoint[]
  /** Ordered investment accounts with display names. */
  accounts: { id: number; name: string }[]
}>()

// ── palette ───────────────────────────────────────────────────────────────────
// Categorical palette for per-account lines. First color reserved for total.
const PALETTE = [
  '#3b82f6', // blue
  '#f59e0b', // amber
  '#8b5cf6', // violet
  '#ef4444', // red
  '#06b6d4', // cyan
  '#ec4899', // pink
  '#84cc16', // lime
]

function resolveColor(cls: string, fallback: string): string {
  const el = document.createElement('span')
  el.className = cls
  el.style.cssText = 'position:absolute;width:0;height:0;visibility:hidden'
  document.body.appendChild(el)
  const c = getComputedStyle(el).color
  document.body.removeChild(el)
  return c || fallback
}

const COLOR_TOTAL = ref('#10b981')
onMounted(() => { COLOR_TOTAL.value = resolveColor('text-primary', COLOR_TOTAL.value) })

const accountColors = computed(() =>
  props.accounts.map((_, i) => PALETTE[i % PALETTE.length])
)

// ── data ──────────────────────────────────────────────────────────────────────
type D = { t: number; total: number; perAccount: number[] }

const data = computed<D[]>(() =>
  props.points.map(p => ({
    t:          DateTime.fromISO(p.date).toMillis(),
    total:      p.total,
    perAccount: props.accounts.map(a => p.byAccount[a.id] ?? 0),
  }))
)

// ── accessors ─────────────────────────────────────────────────────────────────
const x      = (d: D) => d.t
const yTotal = (d: D) => d.total

// One accessor per account — passed as an array to a single VisLine so Unovis
// handles multi-series natively rather than via v-for (which disrupts VisArea).
const yPerAccount = computed(() =>
  props.accounts.map((_, i) => (d: D) => d.perAccount[i])
)

const yAreaPerAccount = computed(() =>
  props.accounts.map((a, i) => ({ id: a.id, y: (d: D) => d.perAccount[i] }))
)

// ── axes ──────────────────────────────────────────────────────────────────────
const tickFormatX = (t: number | Date) =>
  DateTime.fromMillis(typeof t === 'number' ? t : t.getTime()).toFormat('LLL yyyy')

const tickFormatY = (v: number | Date) => {
  const n = typeof v === 'number' ? v : 0
  if (Math.abs(n) >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`
  if (Math.abs(n) >= 1_000)     return `$${(n / 1_000).toFixed(0)}k`
  return `$${n.toFixed(0)}`
}

// ── legend ────────────────────────────────────────────────────────────────────
const legendItems = computed(() => [
  { name: 'Total Investments', color: COLOR_TOTAL.value },
  ...props.accounts.map((a, i) => ({ name: a.name, color: accountColors.value[i] })),
])

// ── tooltip ───────────────────────────────────────────────────────────────────
function money(n: number) {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

const crosshairTemplate = (d: D) => {
  const date = DateTime.fromMillis(d.t).toFormat('MMMM yyyy')
  const totalRow = `<div class="tmfi-chart-tip__row"><span>Total</span><span>${money(d.total)}</span></div>`
  const accountRows = props.accounts.map((a, i) =>
    `<div class="tmfi-chart-tip__row tmfi-chart-tip__row--muted"><span>${a.name}</span><span>${money(d.perAccount[i])}</span></div>`
  ).join('')
  return `<div class="tmfi-chart-tip">
    <div class="tmfi-chart-tip__date">${date}</div>
    ${totalRow}${accountRows}
  </div>`
}
</script>

<template>
  <template v-if="points.length > 0">
    <VisBulletLegend :items="legendItems" class="mb-3" />
    <VisXYContainer :data="data" :height="280" class="tmfi-inv-chart">
      <!-- Total: hero area + line -->
      <VisArea :x="x" :y="yTotal" :color="COLOR_TOTAL" :baseline="0" :opacity="0.08" />
      <VisArea v-for="(a, i) in yAreaPerAccount" :key="a.id" :x="x" :y="a.y" :color="accountColors[i]" :baseline="0" :opacity="0.08" />
      <!-- Per-account lines: single VisLine with array y + color avoids v-for -->
      <VisLine
        v-if="yPerAccount.length > 0"
        :x="x"
        :y="yPerAccount"
        :color="accountColors"
        :line-width="1.5"
      />
      <VisLine :x="x" :y="yTotal" :color="COLOR_TOTAL" :line-width="2" />
      <!-- Axes + interaction -->
      <VisAxis type="x" :tick-format="tickFormatX" />
      <VisAxis type="y" :tick-format="tickFormatY" />
      <VisCrosshair :x="x" :y="yTotal" :color="COLOR_TOTAL" :template="crosshairTemplate" />
      <VisTooltip />
    </VisXYContainer>
  </template>
  <div v-else class="h-[280px] flex items-center justify-center">
    <p class="text-sm text-muted">Add investment account balances to see your portfolio over time.</p>
  </div>
</template>

<style>
.tmfi-inv-chart {
  --vis-crosshair-line-stroke-color: var(--ui-text-dimmed);
  --vis-crosshair-line-stroke-opacity: 0.5;
  --vis-crosshair-circle-stroke-color: #ffffff;
  --vis-tooltip-padding: 0;
  --vis-tooltip-background-color: var(--ui-bg-elevated);
  --vis-tooltip-border-color: var(--ui-border);
  --vis-tooltip-border-radius: 8px;
  --vis-tooltip-box-shadow: 0 4px 12px -2px rgb(0 0 0 / 0.1);
  --vis-tooltip-text-color: var(--ui-text);
}
</style>
