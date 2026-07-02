<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import { DateTime } from 'luxon'
import { useAssetEventsStore } from '../stores/assetEvents'
import { useAccountsStore } from '../stores/accounts'
import { useStorageStore } from '../stores/storage'
import { assetEventKindItems, LIFE_EXPECTANCY_SUGGESTIONS } from '../lib/assets/constants'
import { currentValue } from '../lib/assets/rollups'
import { listAttachments, uploadAttachment, deleteAttachment, openAttachment } from '../lib/api/storage'
import DateInput from './DateInput.vue'
import CurrencyInput from './CurrencyInput.vue'
import ComboboxInput from './ComboboxInput.vue'
import type { AssetEvent } from '../lib/types/AssetEvent'
import type { AssetAttachment } from '../lib/types/AssetAttachment'

const props = defineProps<{
  editing: AssetEvent | null
  presetAccountId?: number | null
}>()
const emit = defineEmits<{ saved: [] }>()

const store = useAssetEventsStore()
const accountsStore = useAccountsStore()
const storageStore = useStorageStore()
const toast = useToast()
const saving = ref(false)
const saveError = ref<string | null>(null)

// Attachment state
const existingAttachments = ref<AssetAttachment[]>([])
const pendingFiles = ref<{ path: string; name: string }[]>([])
const attachmentLoading = ref(false)
const attachmentError = ref<string | null>(null)

onMounted(() => storageStore.load())

const OTHER = '__other__'

const today = DateTime.now().toISODate()!

// Real-estate accounts the event can attach to, plus a free-text escape hatch.
const assetItems = computed(() => [
  ...accountsStore.accounts
    .filter((a) => a.type === 'real_estate' && a.isActive)
    .map((a) => ({ label: a.name, value: String(a.id) })),
  { label: 'Other / type a label…', value: OTHER },
])

// Known free-text labels for autocomplete on the "Other" path.
const knownLabels = computed(() =>
  [...new Set(store.assetEvents.map((e) => e.assetLabel).filter((l): l is string => !!l))],
)

const form = reactive({
  assetSelection: OTHER as string,
  assetLabel: '',
  date: today,
  description: '',
  kind: 'maintenance',
  cost: 0,
  assetValue: null as number | null,
  vendor: '',
  notes: '',
  lifeExpectancy: '',
  linkedTransactionId: null as number | null,
})

const isOther = computed(() => form.assetSelection === OTHER)

// Last known value for the free-text label being entered, so the user can see
// what's currently recorded and update it.
const knownValueForLabel = computed(() => {
  const label = form.assetLabel.trim().toLowerCase()
  if (!label) return null
  const matching = store.assetEvents.filter(
    (e) => e.accountId == null && (e.assetLabel ?? '').trim().toLowerCase() === label,
  )
  return currentValue(matching)
})

function resetForm() {
  form.assetSelection = props.presetAccountId != null ? String(props.presetAccountId) : OTHER
  form.assetLabel = ''
  form.date = today
  form.description = ''
  form.kind = 'maintenance'
  form.cost = 0
  form.assetValue = null
  form.vendor = ''
  form.notes = ''
  form.lifeExpectancy = ''
  form.linkedTransactionId = null
  pendingFiles.value = []
  attachmentError.value = null
}

async function loadAttachments(eventId: number) {
  existingAttachments.value = await listAttachments(eventId).catch(() => [])
}

async function pickFile() {
  const result = await openFileDialog({ multiple: true, directory: false })
  if (!result) return
  const paths = Array.isArray(result) ? result : [result]
  for (const path of paths) {
    const name = path.split('/').pop() ?? path
    if (!pendingFiles.value.some(f => f.path === path)) {
      pendingFiles.value.push({ path, name })
    }
  }
  attachmentError.value = null
}

function removePending(index: number) {
  pendingFiles.value.splice(index, 1)
}

async function removeExistingAttachment(att: AssetAttachment) {
  attachmentLoading.value = true
  attachmentError.value = null
  try {
    await deleteAttachment(att.id)
    existingAttachments.value = existingAttachments.value.filter(a => a.id !== att.id)
  } catch (e) {
    attachmentError.value = String(e)
  } finally {
    attachmentLoading.value = false
  }
}

async function openExistingAttachment(att: AssetAttachment) {
  attachmentLoading.value = true
  attachmentError.value = null
  try {
    await openAttachment(att.id)
  } catch (e) {
    attachmentError.value = String(e)
  } finally {
    attachmentLoading.value = false
  }
}

watch(
  () => [props.editing, props.presetAccountId] as const,
  ([e]) => {
    saveError.value = null
    existingAttachments.value = []
    if (e) {
      form.assetSelection = e.accountId != null ? String(e.accountId) : OTHER
      form.assetLabel = e.assetLabel ?? ''
      form.date = e.date
      form.description = e.description
      form.kind = e.kind
      form.cost = e.cost
      form.assetValue = e.assetValue ?? null
      form.vendor = e.vendor ?? ''
      form.notes = e.notes ?? ''
      form.lifeExpectancy = e.lifeExpectancy ?? ''
      form.linkedTransactionId = e.linkedTransactionId ?? null
      pendingFiles.value = []
      loadAttachments(e.id)
    } else {
      resetForm()
    }
  },
  { immediate: true },
)

async function save() {
  saveError.value = null
  const accountId = isOther.value ? null : Number(form.assetSelection)
  const assetLabel = isOther.value ? form.assetLabel.trim() : null

  if (accountId == null && !assetLabel) {
    saveError.value = 'Pick an asset account or enter a label.'
    return
  }
  if (!form.description.trim()) {
    saveError.value = 'Description is required.'
    return
  }

  const now = DateTime.now().toISO()!
  const base = {
    accountId,
    assetLabel,
    date: form.date,
    description: form.description.trim(),
    kind: form.kind,
    cost: form.cost ?? 0,
    // Value only applies to free-text assets; real estate uses its account balance.
    assetValue: isOther.value ? form.assetValue : null,
    vendor: form.vendor.trim() || null,
    notes: form.notes.trim() || null,
    lifeExpectancy: (form.kind === 'purchase' || form.kind === 'improvement') ? form.lifeExpectancy.trim() || null : null,
    linkedTransactionId: form.linkedTransactionId,
  }

  saving.value = true
  try {
    let eventId: number
    if (props.editing) {
      const updated = await store.update({ id: props.editing.id, ...base, updatedAt: now })
      eventId = updated.id
      toast.add({ title: 'Asset event updated', color: 'success' })
    } else {
      const created = await store.create({ ...base, createdAt: now })
      eventId = created.id
      toast.add({ title: 'Asset event added', color: 'success' })
    }

    const failedUploads: string[] = []
    for (const file of pendingFiles.value) {
      try {
        await uploadAttachment(eventId, file.path)
      } catch {
        failedUploads.push(file.name)
      }
    }
    if (failedUploads.length > 0) {
      toast.add({
        title: 'Event saved, but some attachments failed to upload',
        description: failedUploads.join(', '),
        color: 'warning',
      })
    }

    emit('saved')
  } catch (err) {
    saveError.value = String(err)
    toast.add({ title: 'Failed to save asset event', description: String(err), color: 'error' })
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <form class="space-y-4" @submit.prevent="save">
    <div>
      <p class="text-xs text-muted mb-1">Asset</p>
      <USelect v-model="form.assetSelection" :items="assetItems" class="w-full" />
    </div>

    <div v-if="isOther">
      <p class="text-xs text-muted mb-1">Asset label</p>
      <ComboboxInput
        v-model="form.assetLabel"
        :items="knownLabels"
        placeholder="e.g. 2019 Honda CR-V"
        class="w-full"
      />
    </div>

    <div class="flex gap-3">
      <div class="flex-1">
        <p class="text-xs text-muted mb-1">Date</p>
        <DateInput v-model="form.date" class="w-full" />
      </div>
      <div class="w-40">
        <p class="text-xs text-muted mb-1">Type</p>
        <USelect v-model="form.kind" :items="assetEventKindItems" class="w-full" />
      </div>
    </div>

    <div>
      <p class="text-xs text-muted mb-1">Description</p>
      <UInput v-model="form.description" placeholder="e.g. Roof replacement" class="w-full" />
    </div>

    <div class="flex gap-3">
      <div class="w-40">
        <p class="text-xs text-muted mb-1">Cost</p>
        <CurrencyInput v-model="form.cost" class="w-full" />
      </div>
      <div class="flex-1">
        <p class="text-xs text-muted mb-1">Vendor (optional)</p>
        <UInput v-model="form.vendor" placeholder="e.g. ABC Roofing" class="w-full" />
      </div>
    </div>

    <div v-if="form.kind === 'purchase' || form.kind === 'improvement'">
      <p class="text-xs text-muted mb-1">Life expectancy (optional)</p>
      <ComboboxInput
        v-model="form.lifeExpectancy"
        :items="LIFE_EXPECTANCY_SUGGESTIONS"
        placeholder="e.g. 20–30 years"
        class="w-full"
      />
    </div>

    <div v-if="isOther">
      <p class="text-xs text-muted mb-1">Current value of this asset (optional)</p>
      <CurrencyInput v-model="form.assetValue" class="w-full" />
      <p v-if="knownValueForLabel != null && form.assetValue == null" class="text-xs text-muted mt-1">
        Last recorded value: {{ knownValueForLabel.toLocaleString('en-US', { style: 'currency', currency: 'USD' }) }}
      </p>
    </div>

    <p v-else class="text-xs text-muted">
      Value for real-estate assets is taken from the account's latest balance.
    </p>

    <div>
      <p class="text-xs text-muted mb-1">Notes (optional)</p>
      <UTextarea v-model="form.notes" :rows="2" placeholder="Warranty, details…" class="w-full" />
    </div>

    <!-- Attachment section -->
    <div>
      <p class="text-xs text-muted mb-1">Attachments (optional)</p>

      <!-- Existing saved attachments -->
      <div v-for="att in existingAttachments" :key="att.id"
           class="flex items-center gap-2 mb-2 p-2 rounded-md bg-elevated border border-default">
        <span class="i-ph-paperclip text-muted shrink-0" />
        <button type="button"
                class="flex-1 text-sm text-left truncate hover:text-primary transition-colors"
                :disabled="attachmentLoading"
                @click="openExistingAttachment(att)">
          {{ att.originalName }}
        </button>
        <UBadge v-if="att.provider !== storageStore.provider"
                color="warning" variant="subtle" size="xs"
                :label="`stored in ${att.provider}`" />
        <UButton color="neutral" variant="ghost" size="xs" icon="i-ph-x"
                 :disabled="attachmentLoading"
                 @click="removeExistingAttachment(att)" />
      </div>

      <!-- Pending (not yet uploaded) files -->
      <div v-for="(file, i) in pendingFiles" :key="file.path"
           class="flex items-center gap-2 mb-2 p-2 rounded-md bg-elevated border border-default">
        <span class="i-ph-paperclip text-muted shrink-0" />
        <span class="flex-1 text-sm truncate">{{ file.name }}</span>
        <UBadge color="neutral" variant="subtle" size="xs" label="pending upload" />
        <UButton color="neutral" variant="ghost" size="xs" icon="i-ph-x"
                 @click="removePending(i)" />
      </div>

      <!-- Cross-device credential warning -->
      <div v-if="storageStore.needsCredentials"
           class="flex items-start gap-2 rounded border border-warning bg-warning/10 px-3 py-2 text-xs">
        <span class="i-ph-key-duotone mt-0.5 shrink-0 text-warning" />
        <span class="text-warning">
          {{ storageStore.providerLabel }} credentials aren't set up on this device yet —
          uploads will fail until you add them in
          <router-link to="/settings/sync" class="underline">Settings → Attachment Storage</router-link>.
        </span>
      </div>

      <!-- Pick button (always visible) -->
      <div class="flex items-center gap-3">
        <UButton type="button" color="neutral" variant="outline" size="sm"
                 icon="i-ph-paperclip"
                 @click="pickFile">
          Attach file
        </UButton>
        <p class="text-xs text-muted">
          <span v-if="storageStore.isCloudProvider"
                class="inline-flex items-center gap-1">
            <span class="i-ph-cloud-check text-success" />
            {{ storageStore.syncLabel }}
          </span>
          <span v-else class="inline-flex items-center gap-1">
            <span class="i-ph-device-mobile-slash text-muted" />
            {{ storageStore.syncLabel }}
          </span>
        </p>
      </div>

      <p v-if="attachmentError" class="text-xs text-error mt-1">{{ attachmentError }}</p>
    </div>

    <p v-if="saveError" class="text-sm text-error">{{ saveError }}</p>

    <div class="flex justify-end gap-2 pt-2">
      <UButton type="submit" :loading="saving" :disabled="saving">
        {{ props.editing ? 'Save' : 'Add event' }}
      </UButton>
    </div>
  </form>
</template>
