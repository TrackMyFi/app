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

function isActive(to: string) {
  if (to === '/') return route.path === '/'
  return route.path.startsWith(to)
}

const links = [
  { label: 'Dashboard', to: '/', icon: 'i-ph-squares-four' },
  { label: 'Accounts', to: '/accounts', icon: 'i-ph-wallet' },
  { label: 'Assets', to: '/assets', icon: 'i-ph-wrench' },
  { label: 'Transactions', to: '/transactions', icon: 'i-ph-receipt' },
  { label: 'Paychecks', to: '/paychecks', icon: 'i-ph-money' },
  { label: 'Contributions', to: '/contributions', icon: 'i-ph-piggy-bank' },
  { label: 'Budget', to: '/budget', icon: 'i-ph-calculator' },
  { label: 'Forecast', to: '/forecast', icon: 'i-ph-trend-up' },
  { label: 'Settings', to: '/settings', icon: 'i-ph-gear' },
]
</script>

<template>
  <UApp>
    <div class="flex h-screen">
      <nav v-if="route.name !== 'onboarding'" class="w-56 border-r border-default p-3 flex flex-col">
        <div class="space-y-1 flex-1">
          <div class="flex items-center gap-2 px-3 py-3 mb-2">
            <img src="/logo-icon.svg" alt="TrackMyFI" class="w-6 h-6" />
            <span class="font-semibold text-sm tracking-tight">TrackMyFI</span>
          </div>
          <template v-for="l in links" :key="l.label">
            <div v-if="l.label === 'Settings'" class="border-t border-default mx-2 my-1" />
            <RouterLink
              v-if="l.to"
              :to="l.to"
              :class="[
                'flex items-center gap-2 rounded px-3 py-2',
                isActive(l.to)
                  ? 'bg-primary/10 text-primary font-medium'
                  : 'hover:bg-elevated',
              ]"
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
        </div>
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
