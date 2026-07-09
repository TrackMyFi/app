<script setup lang="ts">
import { ref, watch } from 'vue'
import { DateTime } from 'luxon'
import { useStorageStore } from '../stores/storage'
import { labelForHsaCategory, colorForHsaCategory } from '../lib/hsa/constants'
import { listHsaAttachments, openHsaAttachment } from '../lib/api/storage'
import type { HsaExpense } from '../lib/types/HsaExpense'
import type { HsaAttachment } from '../lib/types/HsaAttachment'

const props = defineProps<{
  expense: HsaExpense
  accountName: string | null
}>()

const emit = defineEmits<{ edit: [expense: HsaExpense] }>()

const storageStore = useStorageStore()

const attachments = ref<HsaAttachment[]>([])
const attachmentsLoading = ref(false)
const attachmentError = ref<string | null>(null)
const openingId = ref<number | null>(null)

watch(
  () => props.expense.id,
  async (id) => {
    attachments.value = []
    attachmentError.value = null
    if (!props.expense.hasAttachment) return
    attachmentsLoading.value = true
    try {
      attachments.value = await listHsaAttachments(id)
    } catch (e) {
      attachmentError.value = String(e)
    } finally {
      attachmentsLoading.value = false
    }
  },
  { immediate: true },
)

async function handleOpen(att: HsaAttachment) {
  openingId.value = att.id
  attachmentError.value = null
  try {
    await openHsaAttachment(att.id)
  } catch (e) {
    attachmentError.value = String(e)
  } finally {
    openingId.value = null
  }
}

function providerMismatch(att: HsaAttachment): boolean {
  return att.provider !== storageStore.provider
}

function attachmentLabel(att: HsaAttachment): string {
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

    <!-- Header: category + date + amount -->
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2 flex-wrap">
          <UBadge :color="colorForHsaCategory(expense.category)" variant="subtle">
            {{ labelForHsaCategory(expense.category) }}
          </UBadge>
          <span class="text-sm text-muted">{{ fmtDate(expense.date) }}</span>
        </div>
        <p v-if="expense.provider" class="text-sm text-muted mt-1 flex items-center gap-1">
          <span class="i-ph-first-aid-kit shrink-0" />
          {{ expense.provider }}
        </p>
      </div>
      <div class="text-right shrink-0">
        <p class="text-2xl font-semibold tabular-nums">{{ money(expense.amount) }}</p>
      </div>
    </div>

    <!-- Reimbursement status -->
    <div
      class="flex items-center gap-2 rounded-lg border p-3 text-sm"
      :class="expense.reimbursed ? 'border-success/30 bg-success/10' : 'border-default bg-elevated'"
    >
      <template v-if="expense.reimbursed">
        <span class="i-ph-check-circle text-success shrink-0" />
        <span>
          Reimbursed<template v-if="expense.reimbursedDate"> on {{ fmtDate(expense.reimbursedDate) }}</template><template v-if="accountName"> from {{ accountName }}</template>
        </span>
      </template>
      <template v-else>
        <span class="i-ph-piggy-bank text-muted shrink-0" />
        <span class="text-muted">Not reimbursed yet — still in your receipt bank</span>
      </template>
    </div>

    <!-- Core fields -->
    <dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
      <dt class="text-muted">Description</dt>
      <dd>{{ expense.description }}</dd>

      <template v-if="expense.person">
        <dt class="text-muted">For</dt>
        <dd>{{ expense.person }}</dd>
      </template>
    </dl>

    <!-- Notes -->
    <div v-if="expense.notes" class="rounded-lg bg-elevated border border-default p-3 text-sm">
      <p class="text-xs text-muted uppercase tracking-wide mb-1">Notes</p>
      <p class="whitespace-pre-wrap">{{ expense.notes }}</p>
    </div>

    <!-- Attachments -->
    <div v-if="expense.hasAttachment || attachments.length > 0">
      <p class="text-xs text-muted uppercase tracking-wide mb-2">Receipts &amp; documents</p>

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

    <!-- Metadata -->
    <div class="border-t border-default pt-3 flex gap-6 text-xs text-muted">
      <span>Added {{ fmtDateTime(expense.createdAt) }}</span>
      <span v-if="expense.updatedAt !== expense.createdAt">Updated {{ fmtDateTime(expense.updatedAt) }}</span>
    </div>

    <!-- Edit action -->
    <div class="flex justify-end pt-1">
      <UButton icon="i-ph-pencil" variant="ghost" color="neutral" @click="emit('edit', expense)">
        Edit expense
      </UButton>
    </div>
  </div>
</template>
