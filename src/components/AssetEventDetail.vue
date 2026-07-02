<script setup lang="ts">
import { ref, watch } from 'vue'
import { DateTime } from 'luxon'
import { useStorageStore } from '../stores/storage'
import { labelForAssetEventKind, colorForAssetEventKind } from '../lib/assets/constants'
import { listAttachments, openAttachment } from '../lib/api/storage'
import type { AssetEvent } from '../lib/types/AssetEvent'
import type { AssetAttachment } from '../lib/types/AssetAttachment'

const props = defineProps<{
  event: AssetEvent
  assetName: string
  relatedEvents: AssetEvent[]
}>()

const emit = defineEmits<{ edit: [event: AssetEvent] }>()

const storageStore = useStorageStore()

const attachments = ref<AssetAttachment[]>([])
const attachmentsLoading = ref(false)
const attachmentError = ref<string | null>(null)
const openingId = ref<number | null>(null)

watch(
  () => props.event.id,
  async (id) => {
    attachments.value = []
    attachmentError.value = null
    if (!props.event.hasAttachment) return
    attachmentsLoading.value = true
    try {
      attachments.value = await listAttachments(id)
    } catch (e) {
      attachmentError.value = String(e)
    } finally {
      attachmentsLoading.value = false
    }
  },
  { immediate: true },
)

async function handleOpen(att: AssetAttachment) {
  openingId.value = att.id
  attachmentError.value = null
  try {
    await openAttachment(att.id)
  } catch (e) {
    attachmentError.value = String(e)
  } finally {
    openingId.value = null
  }
}

function providerMismatch(att: AssetAttachment): boolean {
  return att.provider !== storageStore.provider
}

function attachmentLabel(att: AssetAttachment): string {
  if (providerMismatch(att)) return 'Unavailable'
  return storageStore.isCloudProvider ? 'Download' : 'Open'
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

function fmtDate(iso: string): string {
  return DateTime.fromISO(iso).toLocaleString(DateTime.DATE_MED)
}

function fmtDateTime(iso: string): string {
  return DateTime.fromISO(iso).toLocaleString(DateTime.DATETIME_MED)
}

function formatBytes(bytes: number | null): string {
  if (bytes == null) return ''
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <div class="space-y-5">

    <!-- Header: asset + type + date -->
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2 flex-wrap">
          <UBadge :color="colorForAssetEventKind(event.kind)" variant="subtle">
            {{ labelForAssetEventKind(event.kind) }}
          </UBadge>
          <span class="text-sm text-muted">{{ fmtDate(event.date) }}</span>
        </div>
        <p class="text-sm text-muted mt-1 flex items-center gap-1">
          <span :class="event.accountId != null ? 'i-ph-house' : 'i-ph-tag'" class="shrink-0" />
          {{ assetName }}
        </p>
      </div>
      <div class="text-right shrink-0">
        <p class="text-2xl font-semibold tabular-nums">{{ money(event.cost) }}</p>
      </div>
    </div>

    <!-- Core fields -->
    <dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
      <dt class="text-muted">Description</dt>
      <dd>{{ event.description }}</dd>

      <template v-if="event.vendor">
        <dt class="text-muted">Vendor</dt>
        <dd>{{ event.vendor }}</dd>
      </template>

      <template v-if="event.lifeExpectancy">
        <dt class="text-muted">Life expectancy</dt>
        <dd>{{ event.lifeExpectancy }}</dd>
      </template>

      <template v-if="event.assetValue != null && event.accountId == null">
        <dt class="text-muted">Asset value</dt>
        <dd class="tabular-nums font-medium">{{ money(event.assetValue) }}</dd>
      </template>
    </dl>

    <!-- Notes -->
    <div v-if="event.notes" class="rounded-lg bg-elevated border border-default p-3 text-sm">
      <p class="text-xs text-muted uppercase tracking-wide mb-1">Notes</p>
      <p class="whitespace-pre-wrap">{{ event.notes }}</p>
    </div>

    <!-- Attachments -->
    <div v-if="event.hasAttachment || attachments.length > 0">
      <p class="text-xs text-muted uppercase tracking-wide mb-2">Attachments</p>

      <div v-if="attachmentsLoading" class="text-sm text-muted py-2">Loading…</div>

      <div v-for="att in attachments" :key="att.id"
           class="flex items-center gap-2 mb-2 p-2.5 rounded-md bg-elevated border border-default">
        <span class="i-ph-paperclip text-muted shrink-0" />
        <div class="flex-1 min-w-0">
          <p class="text-sm truncate">{{ att.originalName }}</p>
          <p v-if="att.byteSize" class="text-xs text-muted">{{ formatBytes(Number(att.byteSize)) }}</p>
        </div>
        <UTooltip v-if="providerMismatch(att)"
                  :text="`Stored in '${att.provider}' — configure that storage provider to open`">
          <UButton size="xs" variant="ghost" color="neutral" disabled icon="i-ph-warning">
            Unavailable
          </UButton>
        </UTooltip>
        <UButton v-else
                 size="xs"
                 variant="outline"
                 color="neutral"
                 :loading="openingId === att.id"
                 :disabled="openingId !== null"
                 :icon="storageStore.isCloudProvider ? 'i-ph-cloud-arrow-down' : 'i-ph-arrow-square-out'"
                 @click="handleOpen(att)">
          {{ attachmentLabel(att) }}
        </UButton>
      </div>

      <p v-if="attachmentError" class="text-xs text-error mt-1">{{ attachmentError }}</p>
    </div>

    <!-- Related events -->
    <div v-if="relatedEvents.length > 0">
      <p class="text-xs text-muted uppercase tracking-wide mb-2">Other events for this asset</p>
      <table class="w-full text-sm border border-default rounded-lg overflow-hidden">
        <tbody>
          <tr v-for="e in relatedEvents" :key="e.id" class="border-t border-default first:border-t-0">
            <td class="px-3 py-2 whitespace-nowrap text-muted">{{ fmtDate(e.date) }}</td>
            <td class="px-3 py-2">
              <UBadge :color="colorForAssetEventKind(e.kind)" variant="subtle" size="xs">
                {{ labelForAssetEventKind(e.kind) }}
              </UBadge>
            </td>
            <td class="px-3 py-2 min-w-0">
              <span class="line-clamp-1">{{ e.description }}</span>
            </td>
            <td class="px-3 py-2 text-right tabular-nums whitespace-nowrap">{{ money(e.cost) }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Metadata -->
    <div class="border-t border-default pt-3 flex gap-6 text-xs text-muted">
      <span>Added {{ fmtDateTime(event.createdAt) }}</span>
      <span v-if="event.updatedAt !== event.createdAt">Updated {{ fmtDateTime(event.updatedAt) }}</span>
    </div>

    <!-- Edit action -->
    <div class="flex justify-end pt-1">
      <UButton icon="i-ph-pencil" variant="ghost" color="neutral" @click="emit('edit', event)">
        Edit event
      </UButton>
    </div>
  </div>
</template>
