<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { VisXYContainer, VisLine, VisArea, VisAxis, VisCrosshair, VisTooltip, VisBulletLegend } from '@unovis/vue'
import type { NetWorthPoint } from '../lib/fire/netWorthSeries'
import { DateTime } from 'luxon'

const props = defineProps<{
  points: NetWorthPoint[]
  showLiquidSeries?: boolean
}>()

// ── palette ───────────────────────────────────────────────────────────────────
// Resolve CSS color tokens at mount time so SVG paint values match the active
// theme. Falls back to reasonable hex values if the element isn't on the page.
function resolveColor(cls: string, fallback: string): string {
  const el = document.createElement('span')
  el.className = cls
  el.style.cssText = 'position:absolute;width:0;height:0;visibility:hidden'
  document.body.appendChild(el)
  const c = getComputedStyle(el).color
  document.body.removeChild(el)
  return c || fallback
}

const COLOR_TOTAL    = ref('#10b981')
const COLOR_LIQUID   = ref('#3b82f6')
const COLOR_ILLIQUID = ref('#f59e0b')
const COLOR_EQUITY   = ref('#94a3b8')

onMounted(() => {
  COLOR_TOTAL.value    = resolveColor('text-primary',  COLOR_TOTAL.value)
  COLOR_LIQUID.value   = resolveColor('text-info',     COLOR_LIQUID.value)
  COLOR_ILLIQUID.value = resolveColor('text-warning',  COLOR_ILLIQUID.value)
  COLOR_EQUITY.value   = resolveColor('text-muted',    COLOR_EQUITY.value)
})

// ── data ──────────────────────────────────────────────────────────────────────
type D = { t: number; netWorth: number; lessEquity: number | null; liquid: number; illiquid: number }

const data = computed<D[]>(() =>
  props.points.map(p => ({
    t:         DateTime.fromISO(p.date).toMillis(),
    netWorth:  p.netWorth,
    lessEquity: p.lessEquity,
    liquid:    p.liquid,
    illiquid:  p.illiquid,
  }))
)

const hasLessEquity   = computed(() => props.points.some(p => p.lessEquity !== null))
const showLiquid      = computed(() => props.showLiquidSeries ?? false)

// ── accessors ─────────────────────────────────────────────────────────────────
const x           = (d: D) => d.t
const yTotal      = (d: D) => d.netWorth
const yLessEquity = (d: D) => d.lessEquity ?? d.netWorth
const yLiquid     = (d: D) => d.liquid
const yIlliquid   = (d: D) => d.illiquid

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
const legendItems = computed(() => {
  const items = [{ name: 'Total Net Worth', color: COLOR_TOTAL.value }]
  if (hasLessEquity.value) items.push({ name: 'Excl. home equity', color: COLOR_EQUITY.value })
  if (showLiquid.value) {
    items.push(
      { name: 'Liquid',   color: COLOR_LIQUID.value },
      { name: 'Illiquid', color: COLOR_ILLIQUID.value },
    )
  }
  return items
})

// ── tooltip ───────────────────────────────────────────────────────────────────
function money(n: number) {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

const crosshairTemplate = (d: D) => {
  const date = DateTime.fromMillis(d.t).toFormat('MMMM yyyy')
  let rows = `<div class="tmfi-chart-tip__row"><span>Net worth</span><span>${money(d.netWorth)}</span></div>`
  if (hasLessEquity.value && d.lessEquity !== null)
    rows += `<div class="tmfi-chart-tip__row tmfi-chart-tip__row--muted"><span>Excl. home equity</span><span>${money(d.lessEquity)}</span></div>`
  if (showLiquid.value) {
    rows += `<div class="tmfi-chart-tip__row tmfi-chart-tip__row--muted"><span>Liquid</span><span>${money(d.liquid)}</span></div>`
    rows += `<div class="tmfi-chart-tip__row tmfi-chart-tip__row--muted"><span>Illiquid</span><span>${money(d.illiquid)}</span></div>`
  }
  return `<div class="tmfi-chart-tip">
    <div class="tmfi-chart-tip__date">${date}</div>
    ${rows}
  </div>`
}
</script>

<template>
  <template v-if="points.length > 0">
    <VisBulletLegend :items="legendItems" class="mb-3" />
    <VisXYContainer :data="data" :height="280" class="tmfi-nw-chart">
      <!-- Areas: paint largest-first so smaller ones show through -->
      <VisArea :x="x" :y="yTotal"    :color="COLOR_TOTAL"    :baseline="0" :opacity="0.08" />
      <VisArea v-if="showLiquid" :x="x" :y="yIlliquid" :color="COLOR_ILLIQUID" :baseline="0" :opacity="0.1" />
      <VisArea v-if="showLiquid" :x="x" :y="yLiquid"   :color="COLOR_LIQUID"   :baseline="0" :opacity="0.12" />
      <!-- Lines -->
      <VisLine v-if="showLiquid" :x="x" :y="yIlliquid" :color="COLOR_ILLIQUID" :line-width="1.5" />
      <VisLine v-if="showLiquid" :x="x" :y="yLiquid"   :color="COLOR_LIQUID"   :line-width="1.5" />
      <VisLine v-if="hasLessEquity" :x="x" :y="yLessEquity" :color="COLOR_EQUITY" :line-width="1.5" :line-dash-array="[5, 4]" />
      <VisLine :x="x" :y="yTotal" :color="COLOR_TOTAL" :line-width="2" />
      <!-- Axes + interaction -->
      <VisAxis type="x" :tick-format="tickFormatX" />
      <VisAxis type="y" :tick-format="tickFormatY" />
      <VisCrosshair :x="x" :y="yTotal" :color="COLOR_TOTAL" :template="crosshairTemplate" />
      <VisTooltip />
    </VisXYContainer>
  </template>
  <div v-else class="h-[280px] flex items-center justify-center">
    <p class="text-sm text-muted">Add account balances to see your net worth over time.</p>
  </div>
</template>

<style>
.tmfi-nw-chart {
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

.tmfi-chart-tip {
  padding: 8px 10px;
  min-width: 160px;
}
.tmfi-chart-tip__date {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.03em;
  color: var(--ui-text-muted);
  text-transform: uppercase;
  margin-bottom: 4px;
}
.tmfi-chart-tip__row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  font-family: ui-monospace, SFMono-Regular, monospace;
  font-variant-numeric: tabular-nums;
  font-size: 13px;
  font-weight: 600;
  color: var(--ui-text);
  line-height: 1.7;
}
.tmfi-chart-tip__row--muted {
  font-size: 12px;
  font-weight: 400;
  color: var(--ui-text-muted);
}
</style>
