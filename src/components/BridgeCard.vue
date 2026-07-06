<script setup lang="ts">
import { computed } from 'vue'

type LadderSegmentKind = 'accessible' | 'ladder' | 'gap'

/**
 * Timeline of the bridge span (FI → 59½) segmented by funding source, in
 * chronological order: accessible funds carry the seasoning window, ladder
 * conversions take over after it, accessible surplus backfills what the
 * ladder can't reach, and anything left is an uncovered gap.
 */
export interface LadderViz {
  fiLabel: string
  /** Chronological segments as 0..100 shares of the bridge span. */
  segments: { kind: LadderSegmentKind; pct: number }[]
  /** 0..100 position where seasoned conversions become withdrawable. */
  unlockPct: number
  legend: { kind: LadderSegmentKind; label: string; value: string }[]
}

const SEGMENT_CLASS: Record<LadderSegmentKind, string> = {
  accessible: 'bg-primary',
  ladder: 'bg-info',
  gap: 'bg-warning/25',
}
const DOT_CLASS: Record<LadderSegmentKind, string> = {
  accessible: 'bg-primary',
  ladder: 'bg-info',
  gap: 'bg-warning',
}

const props = defineProps<{
  accessibleLabel: string
  deferredLabel: string
  /** 0..100 share of investable funds that are accessible before 59½. */
  accessiblePct: number
  statusText: string
  statusColor: 'success' | 'warning' | 'muted'
  /** Second read on the bridge assuming a Roth conversion ladder; omitted when a ladder can't help. */
  ladderText?: string
  ladderColor?: 'success' | 'warning'
  /** Funding-source timeline of the bridge span; omitted alongside ladderText. */
  ladderViz?: LadderViz
  /** Fine-print caveat under the status line; omitted when there's nothing to qualify. */
  caveat?: string
}>()

const barWidth = computed(() => `${Math.min(Math.max(props.accessiblePct, 0), 100)}%`)
const statusClass = computed(() =>
  props.statusColor === 'success' ? 'text-success' : props.statusColor === 'warning' ? 'text-warning' : 'text-muted')
const ladderClass = computed(() => props.ladderColor === 'success' ? 'text-success' : 'text-warning')
</script>

<template>
  <div class="border border-default rounded-lg p-4">
    <h2 class="font-semibold mb-1">Bridge to 59½</h2>
    <p class="text-xs text-muted mb-3">Early retirement spends from accessible accounts until retirement accounts unlock</p>

    <!-- Two strategies stacked when a ladder is on the table; just the split bar and status line otherwise -->
    <section :class="ladderText ? 'mt-4 pt-3 border-t border-default' : ''">
      <template v-if="ladderText">
        <h3 class="text-sm font-semibold">Taxable-only drawdown</h3>
        <p class="text-xs text-muted mt-0.5 mb-2">Accessible accounts carry every year from FI to 59½ on their own</p>
      </template>

      <!-- Accessible vs penalty-locked share of investable funds -->
      <div class="h-2.5 rounded-full bg-elevated overflow-hidden">
        <div class="h-full rounded-full bg-primary" :style="{ width: barWidth }" />
      </div>
      <div class="mt-2 flex justify-between text-xs">
        <span>
          <span class="inline-block size-2 rounded-full bg-primary align-middle mr-1.5" aria-hidden="true" />
          <span class="text-muted">Accessible now</span>
          <span class="font-mono tabular-nums font-medium ml-1.5">{{ accessibleLabel }}</span>
        </span>
        <span>
          <span class="inline-block size-2 rounded-full bg-elevated align-middle mr-1.5" aria-hidden="true" />
          <span class="text-muted">Locked until 59½</span>
          <span class="font-mono tabular-nums font-medium ml-1.5">{{ deferredLabel }}</span>
        </span>
      </div>

      <p class="mt-3 text-sm" :class="statusClass">{{ statusText }}</p>
    </section>

    <template v-if="ladderText">
      <section class="mt-4 pt-3 border-t border-default">
        <h3 class="text-sm font-semibold">Roth conversion ladder drawdown</h3>
        <p class="text-xs text-muted mt-0.5">Convert pre-tax funds annually starting at FI — each year's conversion becomes spendable five years later</p>
        <p class="mt-2 text-sm" :class="ladderClass">{{ ladderText }}</p>

        <!-- The bridge span replayed as a timeline: who pays for which years -->
        <div v-if="ladderViz" class="mt-3">
          <div class="flex justify-between text-xs text-muted mb-1.5">
            <span>{{ ladderViz.fiLabel }}</span>
            <span>59½</span>
          </div>
          <div class="relative">
            <div class="h-2.5 rounded-full bg-elevated overflow-hidden flex">
              <div
                v-for="(s, i) in ladderViz.segments"
                :key="i"
                class="h-full shrink-0"
                :class="SEGMENT_CLASS[s.kind]"
                :style="{ width: `${s.pct}%` }"
              />
            </div>
            <!-- Where the first seasoned conversion becomes withdrawable -->
            <div
              class="absolute -top-1 -bottom-1 w-px bg-accented"
              :style="{ left: `${ladderViz.unlockPct}%` }"
              aria-hidden="true"
            />
          </div>
          <div class="mt-2 space-y-1">
            <div v-for="item in ladderViz.legend" :key="item.kind" class="flex justify-between gap-3 text-xs">
              <span>
                <span class="inline-block size-2 rounded-full align-middle mr-1.5" :class="DOT_CLASS[item.kind]" aria-hidden="true" />
                <span class="text-muted">{{ item.label }}</span>
              </span>
              <span class="font-mono tabular-nums font-medium text-right" :class="item.kind === 'gap' ? 'text-warning' : ''">
                {{ item.value }}
              </span>
            </div>
          </div>
        </div>

        <p class="mt-2 text-xs text-muted">Conversions are taxed as ordinary income in the year converted.</p>
      </section>
    </template>

    <p v-if="caveat" class="mt-3 text-xs text-muted">{{ caveat }}</p>
  </div>
</template>
