<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  progress: number // real FI progress %, 0..100+
  reveal: number // 0..1 count-up multiplier
  investableLabel: string // "$340,000"
  goalLabel: string // "$1,000,000"
  fiDateLabel?: string // "Mar 2041"
  yearsToFi?: number | null
}>()

// Quarter-marks of the long road. The destination (FI) is the bar's end.
const CHECKPOINTS = [25, 50, 75]

const reached = computed(() => props.progress >= 100)

// Animated display value — eases up from zero on reveal, lands on the real %.
const shown = computed(() => props.progress * props.reveal)
const fillWidth = computed(() => `${Math.min(shown.value, 100)}%`)

// Warm, motivating, still precise. Tied to the *real* value (not the animated
// one) so the phrase doesn't flicker while the number counts up.
const phase = computed(() => {
  const p = props.progress
  if (p >= 100) return 'Financially independent'
  if (p >= 75) return 'The home stretch'
  if (p >= 50) return 'Past halfway'
  if (p >= 25) return 'Gaining ground'
  if (p > 0) return 'Underway'
  return 'Starting out'
})

const checkpoints = computed(() =>
  CHECKPOINTS.map((at) => ({ at, passed: props.progress >= at })),
)

// Five emerald motes for the rare FI-reached celebration. Fixed positions and
// delays so the sparkle reads as composed, not random.
const motes = [
  { left: '12%', delay: '0ms' },
  { left: '34%', delay: '180ms' },
  { left: '52%', delay: '70ms' },
  { left: '71%', delay: '250ms' },
  { left: '88%', delay: '130ms' },
]
</script>

<template>
  <div
    class="tmfi-rise rounded-lg border p-6 transition-colors duration-300"
    :class="reached ? 'border-primary/40 bg-primary/[0.04]' : 'border-default'"
  >
    <div class="flex items-start justify-between gap-4">
      <div>
        <div class="text-xs font-semibold text-muted uppercase tracking-wider">FI Progress</div>
        <div class="mt-2 flex items-baseline gap-3">
          <span
            class="font-mono font-bold tabular-nums text-5xl leading-none"
            :class="reached ? 'text-primary' : 'text-highlighted'"
          >
            <svg
              v-if="reached"
              class="tmfi-check inline-block w-8 h-8 mr-1 -mb-0.5"
              viewBox="0 0 24 24"
              fill="none"
              aria-hidden="true"
            >
              <path
                d="M5 13l4 4L19 7"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>{{ shown.toFixed(1) }}%
          </span>
          <span class="text-sm font-medium" :class="reached ? 'text-primary' : 'text-muted'">
            {{ phase }}
          </span>
        </div>
      </div>

      <div v-if="fiDateLabel" class="text-right shrink-0">
        <div class="text-xs font-semibold text-muted uppercase tracking-wider">Projected FI</div>
        <div class="mt-1 font-mono font-semibold tabular-nums text-lg text-highlighted">
          {{ fiDateLabel }}
        </div>
        <div v-if="yearsToFi != null" class="text-xs text-muted">
          {{ yearsToFi }} year{{ yearsToFi === 1 ? '' : 's' }} to go
        </div>
      </div>
    </div>

    <!-- Journey track: emerald fill grows from zero, checkpoints mark the road -->
    <div
      class="mt-5 relative"
      role="progressbar"
      :aria-valuenow="Math.round(progress)"
      aria-valuemin="0"
      aria-valuemax="100"
      :aria-label="`FI progress: ${Math.round(progress)} percent toward your goal`"
    >
      <div class="relative h-2.5 rounded-full bg-elevated overflow-hidden">
        <div class="relative h-full rounded-full bg-primary" :style="{ width: fillWidth }">
          <span v-if="reached" class="tmfi-sheen" aria-hidden="true" />
        </div>
      </div>

      <!-- Checkpoints reached punch through the fill as pale waypoints -->
      <span
        v-for="c in checkpoints"
        :key="c.at"
        class="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 size-1.5 rounded-full transition-colors duration-300"
        :class="c.passed ? 'ring-1 ring-primary/50' : 'bg-accented'"
        :style="{ left: `${c.at}%`, ...(c.passed ? { background: 'var(--ui-bg)' } : {}) }"
        aria-hidden="true"
      />

      <!-- The rare FI-reached celebration -->
      <template v-if="reached">
        <span
          v-for="(m, i) in motes"
          :key="`mote-${i}`"
          class="tmfi-mote"
          :style="{ left: m.left, animationDelay: m.delay }"
          aria-hidden="true"
        />
      </template>
    </div>

    <!-- The dollars behind the percentage — grounding for FIRE-literate users -->
    <div class="mt-3 text-xs text-muted">
      <span class="font-mono tabular-nums font-medium text-default">{{ investableLabel }}</span>
      invested toward your
      <span class="font-mono tabular-nums font-medium text-default">{{ goalLabel }}</span> goal
    </div>
  </div>
</template>
