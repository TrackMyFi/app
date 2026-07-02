<script setup lang="ts">
import { computed } from 'vue'
import { DateTime } from 'luxon'
import type { DropdownMenuItem } from '@nuxt/ui'
import { useAccountsStore } from '../stores/accounts'
import { labelForAccountType, isLiability } from '../lib/accountTypes'
import { monthChange, yearToDateChange, type BalanceChange } from '../lib/balances/change'
import type { Account } from '../lib/types/Account'

const props = withDefaults(
  defineProps<{
    title: string
    accounts: Account[]
    menuItems: (account: Account) => DropdownMenuItem[][]
    interactive?: boolean
    busyId?: number | null
  }>(),
  { interactive: false, busyId: null },
)

const emit = defineEmits<{ select: [account: Account] }>()

const store = useAccountsStore()

const balanceMap = computed(
  () => new Map(store.latestBalances.map(b => [b.accountId, b.balance] as const)),
)
const dateMap = computed(
  () => new Map(store.latestBalances.map(b => [b.accountId, b.recordedAt] as const)),
)

const money = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

function balanceLabel(id: number) {
  const b = balanceMap.value.get(id)
  return b != null ? money(b) : '—'
}

function dateLabel(id: number) {
  const d = dateMap.value.get(id)
  if (!d) return null
  const dt = DateTime.fromISO(d)
  return dt.year === DateTime.now().year ? dt.toFormat('MMM d') : dt.toFormat('MMM d, yyyy')
}

function isStale(id: number) {
  const d = dateMap.value.get(id)
  if (!d) return false
  return DateTime.now().diff(DateTime.fromISO(d), 'days').days > 30
}

const now = DateTime.now()

const monthChangeMap = computed(() => new Map(props.accounts.map(a => {
  const current = balanceMap.value.get(a.id)
  return [a.id, current != null ? monthChange(store.allBalances, a.id, current, now) : null] as const
})))

const yearToDateChangeMap = computed(() => new Map(props.accounts.map(a => {
  const current = balanceMap.value.get(a.id)
  return [a.id, current != null ? yearToDateChange(store.allBalances, a.id, current, now) : null] as const
})))

function changeAmountLabel(change: BalanceChange | null) {
  if (!change) return '—'
  if (change.amount === 0) return money(0)
  return change.amount > 0 ? `+${money(change.amount)}` : money(change.amount)
}

function changePercentLabel(change: BalanceChange | null) {
  if (!change || change.percent == null) return null
  const pct = change.percent * 100
  const sign = pct > 0 ? '+' : ''
  return `${sign}${pct.toFixed(1)}%`
}

/** Colors by net-worth impact: for a liability, a growing balance is bad (red), not good. */
function changeColorClass(account: Account, change: BalanceChange | null) {
  if (!change || change.amount === 0) return 'text-muted'
  const helpsNetWorth = isLiability(account.type) ? change.amount < 0 : change.amount > 0
  return helpsNetWorth ? 'text-success' : 'text-error'
}

function activate(account: Account) {
  if (props.interactive) emit('select', account)
}
</script>

<template>
  <section>
    <p class="text-xs text-muted font-medium mb-2">{{ title }}</p>
    <div class="border border-default rounded-lg overflow-hidden">
      <!-- Column headers -->
      <div class="grid grid-cols-[1fr_140px_120px_120px_140px_52px] bg-elevated px-4 py-2 border-b border-default">
        <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
        <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
        <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">1M Change</span>
        <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">YTD Change</span>
        <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
        <span />
      </div>

      <!-- Rows -->
      <div
        v-for="account in accounts"
        :key="account.id"
        class="grid grid-cols-[1fr_140px_120px_120px_140px_52px] items-center px-4 py-3 border-b border-default last:border-b-0"
        :class="interactive
          ? 'group cursor-pointer hover:bg-elevated/50 transition-colors focus-visible:outline-none focus-visible:bg-elevated/50 focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary/40'
          : ''"
        :role="interactive ? 'button' : undefined"
        :tabindex="interactive ? 0 : undefined"
        @click="activate(account)"
        @keydown.enter="activate(account)"
        @keydown.space.prevent="activate(account)"
      >
        <div class="min-w-0">
          <div class="flex items-center gap-1.5 min-w-0">
            <p
              class="text-sm font-medium truncate min-w-0"
              :class="interactive ? 'transition-transform duration-200 group-hover:translate-x-0.5' : 'text-muted'"
            >{{ account.name }}</p>
            <UIcon
              v-if="interactive"
              name="i-ph-arrow-right"
              class="size-3.5 shrink-0 text-dimmed opacity-0 -translate-x-1 transition-all duration-200 group-hover:opacity-100 group-hover:translate-x-0"
              aria-hidden="true"
            />
          </div>
          <p v-if="account.institution" class="text-xs text-muted truncate">{{ account.institution }}</p>
        </div>

        <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>

        <div class="text-right">
          <p class="text-sm font-mono" :class="changeColorClass(account, monthChangeMap.get(account.id) ?? null)">{{ changeAmountLabel(monthChangeMap.get(account.id) ?? null) }}</p>
          <p v-if="changePercentLabel(monthChangeMap.get(account.id) ?? null)" class="text-xs font-mono text-muted">{{ changePercentLabel(monthChangeMap.get(account.id) ?? null) }}</p>
        </div>

        <div class="text-right">
          <p class="text-sm font-mono" :class="changeColorClass(account, yearToDateChangeMap.get(account.id) ?? null)">{{ changeAmountLabel(yearToDateChangeMap.get(account.id) ?? null) }}</p>
          <p v-if="changePercentLabel(yearToDateChangeMap.get(account.id) ?? null)" class="text-xs font-mono text-muted">{{ changePercentLabel(yearToDateChangeMap.get(account.id) ?? null) }}</p>
        </div>

        <div class="text-right">
          <p class="text-sm font-mono" :class="interactive ? 'font-semibold' : 'text-muted'">{{ balanceLabel(account.id) }}</p>
          <p
            v-if="dateLabel(account.id)"
            class="text-xs"
            :class="interactive && isStale(account.id) ? 'text-warning' : 'text-muted'"
            :title="interactive && isStale(account.id) ? 'Balance not updated in over 30 days' : undefined"
          >{{ dateLabel(account.id) }}</p>
        </div>

        <div class="flex justify-end" @click.stop @keydown.stop>
          <UDropdownMenu :items="menuItems(account)">
            <UButton
              size="xs"
              variant="ghost"
              icon="i-ph-dots-three"
              color="neutral"
              aria-label="Account options"
              :loading="busyId === account.id"
              :disabled="busyId !== null && busyId !== account.id"
            />
          </UDropdownMenu>
        </div>
      </div>
    </div>
  </section>
</template>
