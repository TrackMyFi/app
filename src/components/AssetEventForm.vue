<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { useAssetEventsStore } from '../stores/assetEvents'
import { useAccountsStore } from '../stores/accounts'
import { assetEventKindItems } from '../lib/assets/constants'
import { currentValue } from '../lib/assets/rollups'
import DateInput from './DateInput.vue'
import CurrencyInput from './CurrencyInput.vue'
import ComboboxInput from './ComboboxInput.vue'
import type { AssetEvent } from '../lib/types/AssetEvent'

const props = defineProps<{
  editing: AssetEvent | null
  presetAccountId?: number | null
}>()
const emit = defineEmits<{ saved: [] }>()

const store = useAssetEventsStore()
const accountsStore = useAccountsStore()
const toast = useToast()
const saving = ref(false)
const saveError = ref<string | null>(null)

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
  form.linkedTransactionId = null
}

watch(
  () => [props.editing, props.presetAccountId] as const,
  ([e]) => {
    saveError.value = null
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
      form.linkedTransactionId = e.linkedTransactionId ?? null
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
    linkedTransactionId: form.linkedTransactionId,
  }

  saving.value = true
  try {
    if (props.editing) {
      await store.update({ id: props.editing.id, ...base, updatedAt: now })
      toast.add({ title: 'Asset event updated', color: 'success' })
    } else {
      await store.create({ ...base, createdAt: now })
      toast.add({ title: 'Asset event added', color: 'success' })
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

    <p v-if="saveError" class="text-sm text-error">{{ saveError }}</p>

    <div class="flex justify-end gap-2 pt-2">
      <UButton type="submit" :loading="saving" :disabled="saving">
        {{ props.editing ? 'Save' : 'Add event' }}
      </UButton>
    </div>
  </form>
</template>
