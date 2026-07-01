<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { VisXYContainer, VisLine, VisArea, VisAxis, VisScatter, VisCrosshair, VisTooltip } from '@unovis/vue'
import type { ProjectionPoint } from '../lib/fire/projection'
import { DateTime } from 'luxon'

const props = defineProps<{
  points: ProjectionPoint[]
  fireNumber: number
  coastNumber: number
  /** The day the portfolio is projected to reach the FIRE number, if reachable. */
  crossing: { date: string; value: number } | null
}>()

// ── palette ───────────────────────────────────────────────────────────────
// Resolve CSS color tokens at mount time so SVG paint values match the active
// theme, same pattern as NetWorthChart.
function resolveColor(cls: string, fallback: string): string {
  const el = document.createElement('span')
  el.className = cls
  el.style.cssText = 'position:absolute;width:0;height:0;visibility:hidden'
  document.body.appendChild(el)
  const c = getComputedStyle(el).color
  document.body.removeChild(el)
  return c || fallback
}

const COLOR_PORTFOLIO = ref('#10b981')
const COLOR_FIRE = ref('#1e293b')
const COLOR_COAST = ref('#94a3b8')

onMounted(() => {
  COLOR_PORTFOLIO.value = resolveColor('text-primary', COLOR_PORTFOLIO.value)
  COLOR_FIRE.value = resolveColor('text-highlighted', COLOR_FIRE.value)
  COLOR_COAST.value = resolveColor('text-muted', COLOR_COAST.value)
})

type D = { t: number; v: number; fire: number; coast: number }

// Memoized: the template reads `data` on every crosshair mousemove, so parsing
// ~350 ISO dates per move (when this was a function call) made hover janky.
const data = computed<D[]>(() => props.points.map(p => ({
  t: DateTime.fromISO(p.date).toMillis(),
  v: p.value,
  fire: props.fireNumber,
  coast: props.coastNumber,
})))

const x = (d: D) => d.t
const yValue = (d: D) => d.v
const yFire = (d: D) => d.fire
const yCoast = (d: D) => d.coast

const tickFormatX = (t: number | Date) =>
  DateTime.fromMillis(typeof t === 'number' ? t : t.getTime()).toFormat('yyyy')

const tickFormatY = (n: number | Date) => {
  const v = typeof n === 'number' ? n : (n as Date).getTime()
  if (v >= 1_000_000) return '$' + (v / 1_000_000).toFixed(1) + 'M'
  if (v >= 1_000) return '$' + Math.round(v / 1_000) + 'K'
  return '$' + v
}

function money(n: number) {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

// Tooltip leads with the projected investable value, then frames it against the
// goal as a single percentage — more telling than repeating the constant targets
// on every row, and quieter on screen.
const crosshairTemplate = (d: D) => {
  const date = DateTime.fromMillis(d.t).toFormat('LLLL yyyy')
  const pct = d.fire > 0 ? Math.round((d.v / d.fire) * 100) : 0
  return `<div class="tmfi-chart-tip">
    <div class="tmfi-chart-tip__date">${date}</div>
    <div class="tmfi-chart-tip__value">${money(d.v)}</div>
    <div class="tmfi-chart-tip__meta">${pct}% of FIRE target</div>
  </div>`
}

// ── FI crossing marker ────────────────────────────────────────────────────
// The one moment this chart exists to show: the day the curve meets the FIRE
// number. A single clean dot pins it; the label dates it right on the line.
type Mark = { t: number; v: number }
const crossingData = computed<Mark[]>(() =>
  props.crossing ? [{ t: DateTime.fromISO(props.crossing.date).toMillis(), v: props.crossing.value }] : [])
const mx = (d: Mark) => d.t
const my = (d: Mark) => d.v
const markLabel = (d: Mark) => DateTime.fromMillis(d.t).toFormat('LLL yyyy')
</script>

<template>
  <VisXYContainer :data="data" :height="280" class="tmfi-forecast-chart">
    <!-- Investable: the hero. A calm emerald fill grounds a light line, so the
         curve reads as substantial without shouting. -->
    <VisArea :x="x" :y="yValue" :color="COLOR_PORTFOLIO" :baseline="0" :opacity="0.1" />
    <VisLine :x="x" :y="yValue" :color="COLOR_PORTFOLIO" :line-width="2" />
    <!-- Reference thresholds, kept quiet so emerald stays the single voice. -->
    <VisLine :x="x" :y="yFire" :color="COLOR_FIRE" :line-width="1.5" :line-dash-array="[5, 5]" />
    <VisLine :x="x" :y="yCoast" :color="COLOR_COAST" :line-width="1.5" :line-dash-array="[2, 4]" />
    <template v-if="crossing">
      <VisScatter
        :data="crossingData" :x="mx" :y="my"
        :color="COLOR_PORTFOLIO" :size="8"
        :stroke-color="'#ffffff'" :stroke-width="2"
        :label="markLabel" label-position="top" :label-color="COLOR_FIRE"
      />
    </template>
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" :tick-format="tickFormatY" />
    <VisCrosshair :x="x" :y="yValue" :color="COLOR_PORTFOLIO" :template="crosshairTemplate" />
    <VisTooltip />
  </VisXYContainer>
</template>

<style>
.tmfi-forecast-chart {
  /* Crossing label: sits just above the dot on the curve. */
  --vis-scatter-point-label-text-font-size: 11px;
  --vis-scatter-point-label-text-font-weight: 600;
  /* Hover guide: a quiet vertical line, emerald node where it meets the curve. */
  --vis-crosshair-line-stroke-color: var(--ui-text-dimmed);
  --vis-crosshair-line-stroke-opacity: 0.5;
  --vis-crosshair-circle-stroke-color: #ffffff;
  /* Tooltip — flat, token-backed card matching the app's surfaces. */
  --vis-tooltip-padding: 0;
  --vis-tooltip-background-color: var(--ui-bg-elevated);
  --vis-tooltip-border-color: var(--ui-border);
  --vis-tooltip-border-radius: 8px;
  --vis-tooltip-box-shadow: 0 4px 12px -2px rgb(0 0 0 / 0.1);
  --vis-tooltip-text-color: var(--ui-text);
}

/* Tooltip — flat, token-backed card matching the app's surfaces. */
.tmfi-chart-tip {
  padding: 8px 10px;
  min-width: 132px;
}
.tmfi-chart-tip__date {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.03em;
  color: var(--ui-text-muted);
  text-transform: uppercase;
}
.tmfi-chart-tip__value {
  margin-top: 3px;
  font-family: ui-monospace, SFMono-Regular, monospace;
  font-variant-numeric: tabular-nums;
  font-size: 15px;
  font-weight: 600;
  color: var(--ui-primary);
}
.tmfi-chart-tip__meta {
  margin-top: 1px;
  font-size: 11px;
  color: var(--ui-text-dimmed);
}
</style>
