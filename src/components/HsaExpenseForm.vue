<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import { DateTime } from 'luxon'
import { useHsaExpensesStore } from '../stores/hsaExpenses'
import { useAccountsStore } from '../stores/accounts'
import { useStorageStore } from '../stores/storage'
import { hsaCategoryItems } from '../lib/hsa/constants'
import { listHsaAttachments, uploadHsaAttachment, deleteHsaAttachment, openHsaAttachment } from '../lib/api/storage'
import DateInput from './DateInput.vue'
import CurrencyInput from './CurrencyInput.vue'
import ComboboxInput from './ComboboxInput.vue'
import type { HsaExpense } from '../lib/types/HsaExpense'
import type { HsaAttachment } from '../lib/types/HsaAttachment'

const props = defineProps<{
  editing: HsaExpense | null
  presetAccountId?: number | null
}>()
const emit = defineEmits<{ saved: [] }>()

const store = useHsaExpensesStore()
const accountsStore = useAccountsStore()
const storageStore = useStorageStore()
const toast = useToast()
const saving = ref(false)
const saveError = ref<string | null>(null)

// Attachment state
const existingAttachments = ref<HsaAttachment[]>([])
const pendingFiles = ref<{ path: string; name: string }[]>([])
const attachmentLoading = ref(false)
const attachmentError = ref<string | null>(null)

onMounted(() => storageStore.load())

// Sentinel for "no account selected" (Reka UI's <SelectItem> rejects an empty-string value).
const NONE = '__none__'

const today = DateTime.now().toISODate()!

// HSA accounts the expense can be reimbursed from.
const accountItems = computed(() => [
  { label: 'No account', value: NONE },
  ...accountsStore.accounts
    .filter((a) => a.type === 'hsa' && a.isActive)
    .map((a) => ({ label: a.name, value: String(a.id) })),
])

// Known people / providers for autocomplete.
const knownPeople = computed(() =>
  [...new Set(store.hsaExpenses.map((e) => e.person).filter((p): p is string => !!p))],
)
const knownProviders = computed(() =>
  [...new Set(store.hsaExpenses.map((e) => e.provider).filter((p): p is string => !!p))],
)

const form = reactive({
  accountSelection: NONE as string,
  date: today,
  description: '',
  category: 'medical',
  amount: 0,
  person: '',
  provider: '',
  notes: '',
  reimbursed: false,
  reimbursedDate: '' as string,
})

function resetForm() {
  form.accountSelection = props.presetAccountId != null ? String(props.presetAccountId) : NONE
  form.date = today
  form.description = ''
  form.category = 'medical'
  form.amount = 0
  form.person = ''
  form.provider = ''
  form.notes = ''
  form.reimbursed = false
  form.reimbursedDate = ''
  pendingFiles.value = []
  attachmentError.value = null
}

async function loadAttachments(expenseId: number) {
  existingAttachments.value = await listHsaAttachments(expenseId).catch(() => [])
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

async function removeExistingAttachment(att: HsaAttachment) {
  attachmentLoading.value = true
  attachmentError.value = null
  try {
    await deleteHsaAttachment(att.id)
    existingAttachments.value = existingAttachments.value.filter(a => a.id !== att.id)
  } catch (e) {
    attachmentError.value = String(e)
  } finally {
    attachmentLoading.value = false
  }
}

async function openExistingAttachment(att: HsaAttachment) {
  attachmentLoading.value = true
  attachmentError.value = null
  try {
    await openHsaAttachment(att.id)
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
      form.accountSelection = e.accountId != null ? String(e.accountId) : NONE
      form.date = e.date
      form.description = e.description
      form.category = e.category
      form.amount = e.amount
      form.person = e.person ?? ''
      form.provider = e.provider ?? ''
      form.notes = e.notes ?? ''
      form.reimbursed = e.reimbursed
      form.reimbursedDate = e.reimbursedDate ?? ''
      pendingFiles.value = []
      loadAttachments(e.id)
    } else {
      resetForm()
    }
  },
  { immediate: true },
)

// Default the reimbursement date to today when toggling reimbursed on.
watch(() => form.reimbursed, (on) => {
  if (on && !form.reimbursedDate) form.reimbursedDate = today
})

async function save() {
  saveError.value = null
  if (!form.description.trim()) {
    saveError.value = 'Description is required.'
    return
  }

  const now = DateTime.now().toISO()!
  const base = {
    accountId: form.accountSelection === NONE ? null : Number(form.accountSelection),
    date: form.date,
    description: form.description.trim(),
    category: form.category,
    amount: form.amount ?? 0,
    person: form.person.trim() || null,
    provider: form.provider.trim() || null,
    notes: form.notes.trim() || null,
    reimbursed: form.reimbursed,
    reimbursedDate: form.reimbursed ? form.reimbursedDate || null : null,
  }

  saving.value = true
  try {
    let expenseId: number
    if (props.editing) {
      const updated = await store.update({ id: props.editing.id, ...base, updatedAt: now })
      expenseId = updated.id
      toast.add({ title: 'HSA expense updated', color: 'success' })
    } else {
      const created = await store.create({ ...base, createdAt: now })
      expenseId = created.id
      toast.add({ title: 'HSA expense added', color: 'success' })
    }

    const failedUploads: string[] = []
    for (const file of pendingFiles.value) {
      try {
        await uploadHsaAttachment(expenseId, file.path)
      } catch {
        failedUploads.push(file.name)
      }
    }
    if (failedUploads.length > 0) {
      toast.add({
        title: 'Expense saved, but some attachments failed to upload',
        description: failedUploads.join(', '),
        color: 'warning',
      })
    }

    emit('saved')
  } catch (err) {
    saveError.value = String(err)
    toast.add({ title: 'Failed to save HSA expense', description: String(err), color: 'error' })
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <form class="space-y-4" @submit.prevent="save">
    <div class="flex gap-3">
      <div class="flex-1">
        <p class="text-xs text-muted mb-1">Date of service</p>
        <DateInput v-model="form.date" class="w-full" />
      </div>
      <div class="w-40">
        <p class="text-xs text-muted mb-1">Category</p>
        <USelect v-model="form.category" :items="hsaCategoryItems" class="w-full" />
      </div>
    </div>

    <div>
      <p class="text-xs text-muted mb-1">Description</p>
      <UInput v-model="form.description" placeholder="e.g. Annual physical copay" class="w-full" />
    </div>

    <div class="flex gap-3">
      <div class="w-40">
        <p class="text-xs text-muted mb-1">Amount</p>
        <CurrencyInput v-model="form.amount" class="w-full" />
      </div>
      <div class="flex-1">
        <p class="text-xs text-muted mb-1">Provider (optional)</p>
        <ComboboxInput
          v-model="form.provider"
          :items="knownProviders"
          placeholder="e.g. Springfield Family Medicine"
          class="w-full"
        />
      </div>
    </div>

    <div>
      <p class="text-xs text-muted mb-1">Who it was for (optional)</p>
      <ComboboxInput
        v-model="form.person"
        :items="knownPeople"
        placeholder="e.g. Tom"
        class="w-full"
      />
    </div>

    <!-- Reimbursement -->
    <div class="rounded-lg border border-default p-3 space-y-3">
      <USwitch v-model="form.reimbursed" label="Reimbursed from HSA" />
      <template v-if="form.reimbursed">
        <div class="flex gap-3">
          <div class="w-40">
            <p class="text-xs text-muted mb-1">Reimbursed on</p>
            <DateInput v-model="form.reimbursedDate" class="w-full" />
          </div>
          <div class="flex-1">
            <p class="text-xs text-muted mb-1">From account (optional)</p>
            <USelect v-model="form.accountSelection" :items="accountItems" class="w-full" />
          </div>
        </div>
      </template>
      <p v-else class="text-xs text-muted">
        Unreimbursed expenses stay in your receipt bank — you can withdraw the money
        tax-free from your HSA at any time in the future.
      </p>
    </div>

    <div>
      <p class="text-xs text-muted mb-1">Notes (optional)</p>
      <UTextarea v-model="form.notes" :rows="2" placeholder="Claim number, details…" class="w-full" />
    </div>

    <!-- Attachment section -->
    <div>
      <p class="text-xs text-muted mb-1">Receipts &amp; documents (optional)</p>

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
        {{ props.editing ? 'Save' : 'Add expense' }}
      </UButton>
    </div>
  </form>
</template>
