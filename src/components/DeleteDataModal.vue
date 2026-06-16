<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useRouter } from 'vue-router'
import { previewDataDeletion, deleteData } from '../lib/api/dataManagement'
import type { DeletionRange } from '../lib/types/DeletionRange'
import type { DeletionPreview } from '../lib/types/DeletionPreview'

const open = defineModel<boolean>('open', { default: false })

const router = useRouter()

const WORDS = [
  'apple', 'arrow', 'bark', 'beach', 'bird', 'bridge', 'brook', 'cabin',
  'cedar', 'cliff', 'cloud', 'coral', 'crane', 'creek', 'delta', 'dune',
  'eagle', 'ember', 'fern', 'field', 'flame', 'flint', 'fog', 'forest',
  'frost', 'grove', 'haven', 'hawk', 'heath', 'heron', 'hill', 'hollow',
  'inlet', 'iris', 'jade', 'kelp', 'lake', 'lantern', 'lark', 'leaf',
  'ledge', 'lime', 'linden', 'maple', 'marsh', 'meadow', 'mesa', 'mist',
  'moss', 'oak', 'pebble', 'pine', 'pond', 'quartz', 'rain', 'raven',
  'reed', 'ridge', 'river', 'robin', 'rock', 'sage', 'shore', 'slate',
  'spruce', 'stone', 'storm', 'stream', 'summit', 'swift', 'thorn', 'tide',
]

function generateCode() {
  const pick = () => WORDS[Math.floor(Math.random() * WORDS.length)]
  return `${pick()}-${pick()}-${pick()}`
}

type RangeOption = { label: string; range: DeletionRange }

const rangeMap: Record<string, RangeOption> = {
  'days-1':   { label: 'Past 1 day',     range: { type: 'days',   value: 1  } },
  'days-2':   { label: 'Past 2 days',    range: { type: 'days',   value: 2  } },
  'days-3':   { label: 'Past 3 days',    range: { type: 'days',   value: 3  } },
  'days-4':   { label: 'Past 4 days',    range: { type: 'days',   value: 4  } },
  'days-5':   { label: 'Past 5 days',    range: { type: 'days',   value: 5  } },
  'days-6':   { label: 'Past 6 days',    range: { type: 'days',   value: 6  } },
  'days-7':   { label: 'Past 7 days',    range: { type: 'days',   value: 7  } },
  'days-14':  { label: 'Past 14 days',   range: { type: 'days',   value: 14 } },
  'months-1': { label: 'Past 1 month',   range: { type: 'months', value: 1  } },
  'months-2': { label: 'Past 2 months',  range: { type: 'months', value: 2  } },
  'months-3': { label: 'Past 3 months',  range: { type: 'months', value: 3  } },
  'months-6': { label: 'Past 6 months',  range: { type: 'months', value: 6  } },
  'months-9': { label: 'Past 9 months',  range: { type: 'months', value: 9  } },
  'months-12':{ label: 'Past 12 months', range: { type: 'months', value: 12 } },
  'all':      { label: 'Purge all data', range: { type: 'all'               } },
}

const rangeItems = Object.entries(rangeMap).map(([id, { label }]) => ({ label, value: id }))

const step = ref<1 | 2>(1)
const selectedId = ref<string | null>(null)
const resetProfile = ref(false)
const preview = ref<DeletionPreview | null>(null)
const previewLoading = ref(false)
const confirmInput = ref('')
const code = ref('')
const deleting = ref(false)

const selectedOption = computed(() => selectedId.value ? rangeMap[selectedId.value] : null)
const selectedRange = computed(() => selectedOption.value?.range ?? null)
const isPurgeAll = computed(() => selectedId.value === 'all')

function reset() {
  step.value = 1
  selectedId.value = null
  resetProfile.value = false
  preview.value = null
  previewLoading.value = false
  confirmInput.value = ''
  code.value = ''
  deleting.value = false
}

watch(open, (val) => {
  if (!val) reset()
})

watch(selectedId, async (id) => {
  if (!id) { preview.value = null; return }
  const range = rangeMap[id].range
  preview.value = null
  previewLoading.value = true
  try {
    preview.value = await previewDataDeletion(range)
  } catch {
    preview.value = null
  } finally {
    previewLoading.value = false
  }
})

function goToConfirm() {
  code.value = generateCode()
  confirmInput.value = ''
  step.value = 2
}

const codeMatches = computed(() => confirmInput.value === code.value)

async function confirm() {
  if (!selectedRange.value || !codeMatches.value) return
  deleting.value = true
  try {
    await deleteData(selectedRange.value, resetProfile.value)
    open.value = false
    if (selectedRange.value.type === 'all') {
      router.push('/onboarding')
    }
  } finally {
    deleting.value = false
  }
}

function previewSummary(p: DeletionPreview) {
  const parts: string[] = []
  if (p.transactions) parts.push(`${p.transactions} transaction${p.transactions !== 1 ? 's' : ''}`)
  if (p.paychecks) parts.push(`${p.paychecks} paycheck${p.paychecks !== 1 ? 's' : ''}`)
  if (p.balanceSnapshots) parts.push(`${p.balanceSnapshots} balance snapshot${p.balanceSnapshots !== 1 ? 's' : ''}`)
  if (p.budgetMonths) parts.push(`${p.budgetMonths} budget month${p.budgetMonths !== 1 ? 's' : ''}`)
  return parts.length ? parts.join(', ') : 'nothing (no data in this range)'
}
</script>

<template>
  <UModal v-model:open="open" :title="step === 1 ? 'Delete Data' : 'Confirm Deletion'">
    <template #body>
      <!-- Step 1: Select range -->
      <div v-if="step === 1" class="space-y-4">
        <p class="text-sm text-muted">Select how far back you'd like to delete data.</p>

        <USelect
          v-model="selectedId"
          :items="rangeItems"
          placeholder="Select a range…"
          class="w-full"
        />

        <div v-if="previewLoading" class="flex items-center gap-2 text-sm text-muted">
          <UIcon name="i-ph-circle-notch" class="animate-spin" />
          Counting affected records…
        </div>

        <div v-else-if="preview" class="rounded-lg border border-default p-3 text-sm space-y-1">
          <p class="font-medium">This will permanently delete:</p>
          <p class="text-muted">{{ previewSummary(preview) }}</p>
        </div>

        <div v-if="isPurgeAll" class="flex items-center gap-3 pt-1">
          <USwitch v-model="resetProfile" />
          <span class="text-sm">Also reset FIRE profile to defaults</span>
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <UButton variant="ghost" @click="open = false">Cancel</UButton>
          <UButton
            color="error"
            :disabled="!preview || previewLoading"
            @click="goToConfirm"
          >
            Next
          </UButton>
        </div>
      </div>

      <!-- Step 2: Confirm -->
      <div v-else class="space-y-4">
        <div class="rounded-lg border border-default p-3 text-sm space-y-1">
          <p class="font-medium">About to permanently delete:</p>
          <p class="text-muted">{{ preview ? previewSummary(preview) : '' }}</p>
          <p v-if="resetProfile" class="text-muted">FIRE profile will be reset to defaults.</p>
        </div>

        <div class="rounded-lg bg-elevated p-3 text-center">
          <p class="text-xs text-muted mb-1">Type this code to confirm</p>
          <p class="font-mono font-bold text-lg tracking-wide">{{ code }}</p>
        </div>

        <UFormField label="Confirmation code">
          <UInput
            v-model="confirmInput"
            placeholder="Type the code above"
            class="w-full font-mono"
            autofocus
          />
        </UFormField>

        <div class="flex justify-end gap-2 pt-2">
          <UButton variant="ghost" @click="step = 1">Back</UButton>
          <UButton
            color="error"
            :disabled="!codeMatches"
            :loading="deleting"
            @click="confirm"
          >
            Delete
          </UButton>
        </div>
      </div>
    </template>
  </UModal>
</template>
