<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useRouter, useRoute } from 'vue-router'
import { frontendReady } from './lib/api/sync'
import { useSyncStore } from './stores/sync'
import { useFireProfileStore } from './stores/fireProfile'
import { useUpdaterStore } from './stores/updater'
import UpdateNotifier from './components/UpdateNotifier.vue'

const router = useRouter()
const route = useRoute()
const syncStore = useSyncStore()
const fireProfileStore = useFireProfileStore()
const updaterStore = useUpdaterStore()

// Bumped when the backend's background sync catch-up finishes pulling the cloud.
// Keying the active route component on this remounts it, re-running its onMounted
// loaders so freshly-pulled data replaces the last-synced snapshot shown at boot.
// Fires once shortly after launch (before the user interacts), so no active form
// or scroll state is disrupted.
const refreshNonce = ref(0)
let unlistenRefresh: UnlistenFn | undefined

onMounted(async () => {
  syncStore.init()
  updaterStore.init()
  unlistenRefresh = await listen('data-refreshed', () => {
    refreshNonce.value++
  })
  // Listener is now attached — signal the backend it can safely emit the
  // post-catch-up refresh. Without this, the backend could emit before listen()
  // resolved and the refresh would be lost (stale UI until manual navigation).
  frontendReady().catch(() => {})
  try {
    await fireProfileStore.load()
  } catch {
    // load failed; profile stays null — still redirect to onboarding below
  }
  if (!fireProfileStore.profile?.onboardingCompleted) {
    router.push('/onboarding')
  }
})

onUnmounted(() => unlistenRefresh?.())

const navItems = [
  [
    { label: 'Dashboard', icon: 'i-ph-squares-four', to: '/' },
    { label: 'Accounts', icon: 'i-ph-wallet', to: '/accounts' },
    { label: 'Assets', icon: 'i-ph-wrench', to: '/assets' },
    { label: 'Transactions', icon: 'i-ph-receipt', to: '/transactions' },
    { label: 'Expenses', icon: 'i-ph-chart-pie-slice', to: '/expenses' },
    { label: 'Paychecks', icon: 'i-ph-money', to: '/paychecks' },
    { label: 'Contributions', icon: 'i-ph-piggy-bank', to: '/contributions' },
    { label: 'Budget', icon: 'i-ph-calculator', to: '/budget' },
    { label: 'Forecast', icon: 'i-ph-trend-up', to: '/forecast' },
  ],
  [
    {
      value: 'settings',
      label: 'Settings',
      icon: 'i-ph-gear',
      children: [
        { label: 'FIRE Profile', icon: 'i-ph-target', to: '/settings/profile' },
        { label: 'Category Rules', icon: 'i-ph-tag', to: '/settings/category-rules' },
        { label: 'Vendor Rules', icon: 'i-ph-storefront', to: '/settings/vendor-rules' },
        { label: 'Data & Sync', icon: 'i-ph-cloud-arrow-up', to: '/settings/sync' },
        { label: 'General', icon: 'i-ph-gear', to: '/settings/general' },
      ],
    },
  ],
]

// Expands the Settings group by default when landing directly on one of its
// sub-pages (deep link, refresh, or the "complete your profile" prompts).
const navDefaultValue = route.path.startsWith('/settings') ? ['settings'] : []
</script>

<template>
  <UApp>
    <div class="flex h-screen">
      <nav v-if="route.name !== 'onboarding'" class="w-56 border-r border-default p-3 flex flex-col">
        <div class="flex items-center gap-2 px-3 py-3 mb-2">
          <img src="/logo-icon.svg" alt="TrackMyFI" class="w-6 h-6" />
          <span class="font-semibold text-sm tracking-tight">TrackMyFI</span>
        </div>
        <UNavigationMenu
          :items="navItems"
          orientation="vertical"
          color="primary"
          :default-value="navDefaultValue"
          class="flex-1"
        />
        <UpdateNotifier />
      </nav>
      <main class="flex-1 overflow-auto">
        <RouterView v-slot="{ Component }">
          <component :is="Component" :key="`${route.fullPath}:${refreshNonce}`" />
        </RouterView>
      </main>
    </div>
  </UApp>
</template>
