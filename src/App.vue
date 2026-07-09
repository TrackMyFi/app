<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useRouter, useRoute } from 'vue-router'
import { frontendReady } from './lib/api/sync'
import { useSyncStore } from './stores/sync'
import { useFireProfileStore } from './stores/fireProfile'
import { useUpdaterStore } from './stores/updater'
import UpdateNotifier from './components/UpdateNotifier.vue'
import AccountsNavPanel from './components/AccountsNavPanel.vue'
import { useSyncNotifications } from './composables/useSyncNotifications'

const router = useRouter()
const route = useRoute()
const syncStore = useSyncStore()
const fireProfileStore = useFireProfileStore()
const updaterStore = useUpdaterStore()

// Toasts announcing background cloud/bank syncs while they run.
useSyncNotifications()

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

type NavContext = 'budget' | 'tracking'

const contextTabs = [
  { label: 'Budget', value: 'budget' },
  { label: 'Tracking', value: 'tracking' },
]

const budgetPages = [
  { label: 'Accounts', icon: 'i-ph-wallet', to: '/accounts' },
  { label: 'Transactions', icon: 'i-ph-receipt', to: '/transactions' },
  { label: 'Expenses', icon: 'i-ph-chart-pie-slice', to: '/expenses' },
  { label: 'Paychecks', icon: 'i-ph-money', to: '/paychecks' },
  { label: 'Budget', icon: 'i-ph-calculator', to: '/budget' },
]

const trackingPages = [
  { label: 'Progress', icon: 'i-ph-gauge', to: '/' },
  { label: 'Forecast', icon: 'i-ph-trend-up', to: '/forecast' },
  { label: 'Contributions', icon: 'i-ph-piggy-bank', to: '/contributions' },
  { label: 'Assets', icon: 'i-ph-wrench', to: '/assets' },
  { label: 'HSA Receipts', icon: 'i-ph-first-aid-kit', to: '/hsa' },
]

// Which context tab owns each page — used to auto-follow navigation so the
// tab always reflects where you are. Settings has no owner: it sits outside
// the Budget/Tracking split, so visiting it leaves the active tab untouched.
function contextForPath(path: string): NavContext | null {
  if (path.startsWith('/accounts')) return 'budget'
  if (path === '/transactions' || path === '/expenses' || path === '/paychecks' || path === '/budget') return 'budget'
  if (path === '/' || path === '/assets' || path === '/hsa' || path === '/contributions' || path === '/forecast') return 'tracking'
  return null
}

const activeContext = ref<NavContext>(contextForPath(route.path) ?? 'tracking')

watch(() => route.path, path => {
  const context = contextForPath(path)
  if (context) activeContext.value = context
})

const navItems = computed(() => [activeContext.value === 'budget' ? budgetPages : trackingPages])
</script>

<template>
  <UApp>
    <div class="flex h-screen">
      <nav v-if="route.name !== 'onboarding'" class="w-64 border-r border-default p-3 flex flex-col h-screen">
        <div class="flex items-center justify-between pl-3 py-3 mb-2 shrink-0">
          <div class="flex items-center gap-2">
            <img src="/logo-icon.svg" alt="TrackMyFI" class="w-6 h-6" />
            <span class="font-semibold text-sm tracking-tight">TrackMyFI</span>
          </div>
          <UButton to="/settings/profile" icon="i-ph-gear" variant="ghost" color="neutral" size="sm" aria-label="Settings" />
        </div>
        <UTabs
          v-model="activeContext"
          :items="contextTabs"
          color="neutral"
          variant="pill"
          size="sm"
          :content="false"
          class="shrink-0 mb-2"
        />
        <UNavigationMenu
          :items="navItems"
          orientation="vertical"
          color="primary"
          class="shrink-0"
        />
        <AccountsNavPanel />
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
