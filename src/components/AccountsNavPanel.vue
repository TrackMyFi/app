<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useAccountsStore } from '../stores/accounts'
import { groupAccounts, netWorth } from '../lib/accounts/groups'

const store = useAccountsStore()
const route = useRoute()

onMounted(() => { store.loadList() })

const balanceMap = computed(() => new Map(store.latestBalances.map(b => [b.accountId, b.balance])))
const balanceOf = (accountId: number) => balanceMap.value.get(accountId) ?? 0

const groups = computed(() => groupAccounts(store.accounts, balanceOf))
const netWorthValue = computed(() => netWorth(store.accounts, balanceOf))

const activeAccountId = computed(() =>
  route.name === 'account-detail' ? Number(route.params.id) : null,
)

const fmt = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
</script>

<template>
  <div class="flex-1 min-h-0 flex flex-col mt-3 pt-5">
    <div class="flex-1 min-h-0 overflow-y-auto px-1">
      <p v-if="groups.length === 0" class="px-2 text-xs text-muted">No accounts yet</p>

      <div
        v-for="group in groups"
        :key="group.key"
        class="mb-3 last:mb-0 rounded-lg border border-default/60 overflow-hidden"
      >
        <div class="flex items-center justify-between px-2.5 py-1.5 bg-elevated border-b border-default/60">
          <span class="text-xs font-bold uppercase tracking-wider text-default">{{ group.label }}</span>
          <span class="text-xs font-mono font-semibold text-default">{{ fmt(group.total) }}</span>
        </div>
        <!-- SimpleFIN isn't wired up yet; once it is, a per-account connection
             status dot (synced/error/not-linked) belongs on this row. -->
        <RouterLink
          v-for="account in group.accounts"
          :key="account.id"
          :to="{ name: 'account-detail', params: { id: account.id } }"
          class="flex items-center justify-between gap-2 px-2.5 py-1.5 text-sm transition-colors hover:bg-elevated"
          :class="activeAccountId === account.id ? 'bg-elevated text-primary font-medium' : 'text-default'"
        >
          <span class="truncate">{{ account.name }}</span>
          <span class="font-mono text-xs shrink-0" :class="activeAccountId === account.id ? 'text-primary' : 'text-muted'">
            {{ fmt(balanceOf(account.id)) }}
          </span>
        </RouterLink>
      </div>
    </div>

    <div class="flex items-center justify-between px-2 pt-3 mt-1 border-t border-default">
      <span class="text-xs font-semibold uppercase tracking-wider text-muted">Net Worth</span>
      <span class="text-sm font-mono font-semibold">{{ fmt(netWorthValue) }}</span>
    </div>
  </div>
</template>
