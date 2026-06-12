<script setup lang="ts">
const links = [
  { label: 'Dashboard', to: '/', icon: 'i-lucide-layout-dashboard' },
  { label: 'Accounts', to: '/accounts', icon: 'i-lucide-wallet' },
  { label: 'Transactions', icon: 'i-lucide-receipt', disabled: true },
  { label: 'Paychecks', icon: 'i-lucide-banknote', disabled: true },
  { label: 'Contributions', icon: 'i-lucide-piggy-bank', disabled: true },
  { label: 'Budget', icon: 'i-lucide-calculator', disabled: true },
  { label: 'Forecast', icon: 'i-lucide-trending-up', disabled: true },
  { label: 'Settings', to: '/settings', icon: 'i-lucide-settings' },
]
</script>

<template>
  <UApp>
    <div class="flex h-screen">
      <nav class="w-56 border-r border-default p-3 space-y-1">
        <template v-for="l in links" :key="l.label">
          <RouterLink
            v-if="l.to"
            :to="l.to"
            class="flex items-center gap-2 rounded px-3 py-2 hover:bg-elevated"
          >
            <UIcon :name="l.icon" /> {{ l.label }}
          </RouterLink>
          <span
            v-else
            class="flex items-center gap-2 rounded px-3 py-2 text-muted opacity-50 cursor-not-allowed"
          >
            <UIcon :name="l.icon" /> {{ l.label }}
          </span>
        </template>
      </nav>
      <main class="flex-1 overflow-auto">
        <RouterView />
      </main>
    </div>
  </UApp>
</template>
