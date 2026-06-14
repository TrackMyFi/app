<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useSyncStore } from './stores/sync'
import { useFireProfileStore } from './stores/fireProfile'

const router = useRouter()
const route = useRoute()
const syncStore = useSyncStore()
const fireProfileStore = useFireProfileStore()

onMounted(async () => {
  syncStore.init()
  try {
    await fireProfileStore.load()
  } catch {
    // load failed; profile stays null — still redirect to onboarding below
  }
  if (!fireProfileStore.profile?.onboardingCompleted) {
    router.push('/onboarding')
  }
})

const links = [
  { label: 'Dashboard', to: '/', icon: 'i-lucide-layout-dashboard' },
  { label: 'Accounts', to: '/accounts', icon: 'i-lucide-wallet' },
  { label: 'Transactions', to: '/transactions', icon: 'i-lucide-receipt' },
  { label: 'Paychecks', to: '/paychecks', icon: 'i-lucide-banknote' },
  { label: 'Contributions', to: '/contributions', icon: 'i-lucide-piggy-bank' },
  { label: 'Budget', to: '/budget', icon: 'i-lucide-calculator' },
  { label: 'Forecast', to: '/forecast', icon: 'i-lucide-trending-up' },
  { label: 'Settings', to: '/settings', icon: 'i-lucide-settings' },
]
</script>

<template>
  <UApp>
    <div class="flex h-screen">
      <nav v-if="route.name !== 'onboarding'" class="w-56 border-r border-default p-3 space-y-1">
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
