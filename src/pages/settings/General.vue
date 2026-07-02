<script setup lang="ts">
import { ref, computed } from 'vue'
import { DateTime } from 'luxon'
import { useUpdaterStore, CHECK_INTERVAL_MS } from '../../stores/updater'
import DeleteDataModal from '../../components/DeleteDataModal.vue'
import SettingsNav from '../../components/SettingsNav.vue'

const updater = useUpdaterStore()
const upToDate = ref(false)

const lastCheckedText = computed(() => {
  if (!updater.lastCheckedAt) return '—'
  return DateTime.fromMillis(updater.lastCheckedAt).toLocaleString(DateTime.TIME_SIMPLE)
})

const nextCheckText = computed(() => {
  if (!updater.lastCheckedAt) return '—'
  return DateTime.fromMillis(updater.lastCheckedAt + CHECK_INTERVAL_MS).toLocaleString(DateTime.TIME_SIMPLE)
})

async function checkForUpdates() {
  upToDate.value = false
  await updater.check()
  // The popover handles the "available" case; surface the all-clear here.
  if (updater.status === 'idle') upToDate.value = true
}

const showDeleteModal = ref(false)
</script>

<template>
  <div class="p-6 max-w-3xl">
    <h1 class="text-2xl font-bold mb-4">Settings</h1>
    <SettingsNav />

    <div class="space-y-8">
      <section class="space-y-3">
        <h2 class="text-xl font-bold">Updates</h2>
        <div class="border border-default rounded-lg p-4 space-y-3">
          <div class="flex items-center justify-between gap-4">
            <div>
              <p class="text-sm font-medium">
                TrackMyFI
                <span class="text-muted font-normal">
                  v{{ updater.currentVersion || '—' }}
                </span>
              </p>
              <p class="text-sm text-muted">
                <template v-if="updater.status === 'available'">
                  Version {{ updater.newVersion }} is available.
                </template>
                <template v-else-if="updater.status === 'ready'">
                  Update installed — restart to apply.
                </template>
                <template v-else-if="upToDate">You're on the latest version.</template>
                <template v-else>Check GitHub Releases for a newer version.</template>
              </p>
            </div>
            <UButton
              variant="soft"
              icon="i-ph-arrow-clockwise"
              :loading="updater.status === 'checking'"
              @click="checkForUpdates"
            >
              Check for updates
            </UButton>
          </div>
          <div class="flex gap-6 text-xs text-muted pt-1">
            <span>Last checked: {{ lastCheckedText }}</span>
            <span>Next automatic check: {{ nextCheckText }}</span>
          </div>
        </div>
      </section>

      <hr class="border-default" />

      <section class="space-y-3">
        <h2 class="text-xl font-bold text-error">Danger Zone</h2>
        <div class="border border-error rounded-lg p-4 space-y-3">
          <div>
            <p class="text-sm font-medium">Delete data</p>
            <p class="text-sm text-muted">Permanently remove transactions, paychecks, balance snapshots, and budget months for a selected time range. This cannot be undone.</p>
          </div>
          <UButton color="error" variant="soft" @click="showDeleteModal = true">Delete Data</UButton>
        </div>
      </section>
    </div>

    <DeleteDataModal v-model:open="showDeleteModal" />
  </div>
</template>
