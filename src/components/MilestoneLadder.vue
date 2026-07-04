<script setup lang="ts">
export interface MilestoneRow {
  key: string
  label: string
  targetLabel: string
  achieved: boolean
  /** The first unachieved milestone — the one currently being climbed toward. */
  next: boolean
  /** "Mar 2029 · 3 yrs" on the current trajectory; null when achieved or unreachable. */
  dateLabel: string | null
}

defineProps<{ milestones: MilestoneRow[] }>()

const icon = (m: MilestoneRow) =>
  m.achieved ? 'i-ph-check-circle-fill' : m.next ? 'i-ph-circle-dashed' : 'i-ph-circle'
</script>

<template>
  <div class="border border-default rounded-lg p-4">
    <h2 class="font-semibold mb-1">Milestones</h2>
    <p class="text-xs text-muted mb-3">The waypoints of the long game, on your current trajectory</p>
    <div class="relative">
      <!-- Connecting rail behind the milestone markers -->
      <span class="absolute left-[9px] top-3 bottom-3 w-px bg-accented" aria-hidden="true" />
      <ol>
        <li
          v-for="m in milestones"
          :key="m.key"
          class="relative flex items-center gap-3 py-2"
        >
          <UIcon
            :name="icon(m)"
            class="relative size-5 shrink-0 rounded-full bg-default"
            :class="m.achieved ? 'text-success' : m.next ? 'text-primary' : 'text-muted'"
          />
          <div class="flex-1 min-w-0 flex items-baseline gap-2">
            <span class="text-sm truncate" :class="m.next ? 'font-semibold' : m.achieved ? '' : 'text-muted'">
              {{ m.label }}
            </span>
            <span v-if="m.next" class="text-[10px] font-semibold uppercase tracking-wider text-primary shrink-0">Next</span>
          </div>
          <span class="font-mono tabular-nums text-sm" :class="m.achieved ? '' : 'text-muted'">
            {{ m.targetLabel }}
          </span>
          <span
            class="w-36 text-right text-xs font-mono tabular-nums shrink-0"
            :class="m.achieved ? 'text-success' : 'text-muted'"
          >
            {{ m.achieved ? 'Reached' : m.dateLabel ?? '—' }}
          </span>
        </li>
      </ol>
    </div>
  </div>
</template>
