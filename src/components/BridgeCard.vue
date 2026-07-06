<script setup lang="ts">
import { computed, ref } from 'vue'

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

const showAccessInfo = ref(false)
const showLadderInfo = ref(false)

/** When each pot of money unlocks for an early retiree, in chronological order. */
const ACCESS_TIMELINE = [
  {
    when: 'Anytime',
    what: 'Taxable brokerage, savings, cash',
    detail: 'Spend whenever you like — selling investments may owe capital gains tax, but there\'s no penalty or age gate.',
  },
  {
    when: 'Anytime',
    what: 'Roth IRA contributions',
    detail: 'The dollars you put in directly (not earnings, not conversions) come out tax- and penalty-free at any age. TrackMyFI counts your tracked Roth IRA contributions as accessible; earnings — and any contributions made before you started tracking — stay on the locked side.',
  },
  {
    when: 'At retirement',
    what: 'Roth 401(k) contributions',
    detail: 'Rolling a Roth 401(k) into a Roth IRA — the standard move after leaving a job — turns its contributions into Roth IRA basis, withdrawable immediately. TrackMyFI assumes this rollover happens at FI and counts your tracked Roth-side 401(k) contributions as spendable from then on.',
  },
  {
    when: '5 yrs after converting',
    what: 'Roth conversions',
    detail: 'Each amount converted from pre-tax to Roth becomes withdrawable penalty-free five tax years after its conversion — the Roth conversion ladder.',
  },
  {
    when: 'Age 55',
    what: 'Current employer\'s 401(k)',
    detail: 'The rule of 55: leave that job in or after the year you turn 55 and that plan\'s balance is penalty-free. Doesn\'t apply to IRAs or old employers\' plans.',
  },
  {
    when: 'Age 59½',
    what: 'Everything',
    detail: 'All 401(k)s and IRAs are penalty-free. Roth IRA earnings are also tax-free once the account has been open five years.',
  },
  {
    when: 'Age 65',
    what: 'HSA, for any purpose',
    detail: 'Non-medical HSA withdrawals are penalty-free (taxed as income) from 65. Qualified medical expenses are tax-free at any age.',
  },
]

/** The ladder, step by step — what to actually do. */
const LADDER_STEPS = [
  {
    title: 'Retire into a low-income year',
    detail: 'Conversions are taxed as ordinary income, so the ladder works best once your paycheck stops and your tax bracket drops.',
  },
  {
    title: 'Roll your 401(k) over',
    detail: 'Once you\'ve left the employer, the pre-tax side rolls into a traditional IRA so you can convert on your own schedule. Any Roth side rolls into your Roth IRA, where its contributions become immediately withdrawable basis — a bonus pot for the seasoning years.',
  },
  {
    title: 'Convert about one year of expenses to Roth',
    detail: 'Move that amount from the traditional IRA into a Roth IRA and pay income tax on it that year — ideally at a much lower rate than when you earned it.',
  },
  {
    title: 'Wait five tax years',
    detail: 'Each conversion has its own five-year clock. Until the first one matures, you live on taxable, cash, and other accessible money.',
  },
  {
    title: 'Repeat every year',
    detail: 'Year one\'s conversion is spendable in year six, year two\'s in year seven — a rung matures every year for as long as you keep converting.',
  },
]

const barWidth = computed(() => `${Math.min(Math.max(props.accessiblePct, 0), 100)}%`)
const statusClass = computed(() =>
  props.statusColor === 'success' ? 'text-success' : props.statusColor === 'warning' ? 'text-warning' : 'text-muted')
const ladderClass = computed(() => props.ladderColor === 'success' ? 'text-success' : 'text-warning')
</script>

<template>
  <div class="border border-default rounded-lg p-4">
    <div class="flex items-center gap-1 mb-1">
      <h2 class="font-semibold">Bridge to 59½</h2>
      <UButton
        icon="i-ph-info"
        color="neutral"
        variant="ghost"
        size="xs"
        aria-label="When your money unlocks"
        @click="showAccessInfo = true"
      />
    </div>
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
        <div class="flex items-center gap-1">
          <h3 class="text-sm font-semibold">Roth conversion ladder drawdown</h3>
          <UButton
            icon="i-ph-info"
            color="neutral"
            variant="ghost"
            size="xs"
            aria-label="How a Roth conversion ladder works"
            @click="showLadderInfo = true"
          />
        </div>
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

    <!-- When each pot of money unlocks -->
    <UModal v-model:open="showAccessInfo" title="When your money unlocks">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm text-muted">
            "Locked until 59½" is the conservative summary — several doors open earlier.
            In chronological order:
          </p>
          <ol class="space-y-3">
            <li v-for="row in ACCESS_TIMELINE" :key="row.when + row.what">
              <div class="flex items-baseline gap-2">
                <span class="font-mono tabular-nums text-xs text-primary shrink-0">{{ row.when }}</span>
                <span class="text-sm font-semibold">{{ row.what }}</span>
              </div>
              <p class="mt-0.5 text-sm text-muted">{{ row.detail }}</p>
            </li>
          </ol>
          <p class="text-xs text-muted">
            Withdrawing retirement money before its door opens generally costs a 10% penalty on top of any income tax.
            Other niche routes exist (72(t) SEPP payments, hardship rules) — worth professional advice before relying on them.
          </p>
        </div>
      </template>
    </UModal>

    <!-- How the ladder works, step by step -->
    <UModal v-model:open="showLadderInfo" title="The Roth conversion ladder">
      <template #body>
        <div class="space-y-4">
          <p class="text-sm text-muted">
            A Roth conversion ladder moves pre-tax money — 401(k)s and traditional IRAs — out from
            behind the 59½ penalty wall, five years at a time. It's how early retirees spend
            "locked" money in their 40s without the 10% penalty.
          </p>
          <ol class="space-y-3">
            <li v-for="(step, i) in LADDER_STEPS" :key="step.title" class="flex gap-3">
              <span class="font-mono tabular-nums text-xs text-primary shrink-0 mt-0.5">{{ i + 1 }}</span>
              <div>
                <p class="text-sm font-semibold">{{ step.title }}</p>
                <p class="mt-0.5 text-sm text-muted">{{ step.detail }}</p>
              </div>
            </li>
          </ol>
          <p class="text-xs text-muted">
            The trade-offs: you need five years of spending covered from accessible money before the
            first rung matures, and every conversion adds taxable income for that year — which can
            affect things like ACA health-insurance subsidies.
          </p>
        </div>
      </template>
    </UModal>
  </div>
</template>
