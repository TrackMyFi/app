<script setup lang="ts">
import { computed } from 'vue'
import { useUpdaterStore } from '../stores/updater'

const updater = useUpdaterStore()

// Show the card for everything except the quiet states (idle/checking), and
// hide the "available" prompt once the user has dismissed it for this version.
const visible = computed(() => {
  if (updater.status === 'available' || updater.status === 'error') {
    return !updater.dismissed
  }
  return updater.status === 'downloading' || updater.status === 'ready'
})
</script>

<template>
  <Transition
    enter-active-class="transition duration-200 ease-out"
    enter-from-class="opacity-0 translate-y-2"
    leave-active-class="transition duration-150 ease-in"
    leave-to-class="opacity-0 translate-y-2"
  >
    <div
      v-if="visible"
      class="fixed bottom-4 right-4 z-50 w-80 rounded-lg border border-default bg-default shadow-lg p-4 space-y-3"
    >
      <!-- Update available -->
      <template v-if="updater.status === 'available'">
        <div class="flex items-start gap-2">
          <UIcon name="i-ph-arrow-circle-up" class="text-info text-xl shrink-0 mt-0.5" />
          <div class="min-w-0">
            <p class="font-semibold text-sm">Update available</p>
            <p class="text-xs text-muted">
              Version {{ updater.newVersion }} is ready to install.
            </p>
            <p v-if="updater.notes" class="text-xs text-muted mt-1 line-clamp-3 whitespace-pre-line">
              {{ updater.notes }}
            </p>
          </div>
        </div>
        <div class="flex justify-end gap-2">
          <UButton size="xs" variant="ghost" color="neutral" @click="updater.dismiss()">
            Later
          </UButton>
          <UButton size="xs" icon="i-ph-download-simple" @click="updater.install()">
            Download & install
          </UButton>
        </div>
      </template>

      <!-- Downloading -->
      <template v-else-if="updater.status === 'downloading'">
        <div class="flex items-center gap-2">
          <UIcon name="i-ph-download-simple" class="text-info text-xl shrink-0" />
          <p class="font-semibold text-sm">Downloading update…</p>
        </div>
        <UProgress :model-value="updater.progress ?? undefined" size="sm" />
        <p class="text-xs text-muted text-right">
          {{ updater.progress != null ? `${updater.progress}%` : 'Starting…' }}
        </p>
      </template>

      <!-- Ready to relaunch -->
      <template v-else-if="updater.status === 'ready'">
        <div class="flex items-start gap-2">
          <UIcon name="i-ph-check-circle" class="text-success text-xl shrink-0 mt-0.5" />
          <div>
            <p class="font-semibold text-sm">Update installed</p>
            <p class="text-xs text-muted">Restart to finish updating.</p>
          </div>
        </div>
        <div class="flex justify-end">
          <UButton size="xs" icon="i-ph-arrow-clockwise" @click="updater.restart()">
            Restart now
          </UButton>
        </div>
      </template>

      <!-- Error -->
      <template v-else-if="updater.status === 'error'">
        <div class="flex items-start gap-2">
          <UIcon name="i-ph-warning-circle" class="text-error text-xl shrink-0 mt-0.5" />
          <div class="min-w-0">
            <p class="font-semibold text-sm">Update failed</p>
            <p class="text-xs text-muted break-words">{{ updater.error }}</p>
          </div>
        </div>
        <div class="flex justify-end gap-2">
          <UButton size="xs" variant="ghost" color="neutral" @click="updater.dismiss()">
            Dismiss
          </UButton>
          <UButton size="xs" icon="i-ph-arrow-clockwise" @click="updater.check()">
            Try again
          </UButton>
        </div>
      </template>
    </div>
  </Transition>
</template>
