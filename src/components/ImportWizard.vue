<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { parseCsv } from '../lib/csv/parse'
import { applyMapping, detectDuplicates, type MappingConfig } from '../lib/csv/mapping'
import { bulkCreateTransactions } from '../lib/api/transactions'
import * as mappingApi from '../lib/api/importMappings'
import { useAccountsStore } from '../stores/accounts'
import { useTransactionsStore } from '../stores/transactions'
import type { ImportMapping } from '../lib/types/ImportMapping'

const emit = defineEmits<{ done: [] }>()
const accountsStore = useAccountsStore()
const txnStore = useTransactionsStore()

const step = ref<1 | 2 | 3>(1)
const accountId = ref<number | undefined>(undefined)
const headers = ref<string[]>([])
const rawRows = ref<Record<string, string>[]>([])
const savedMappings = ref<ImportMapping[]>([])
const newMappingName = ref('')

const config = ref<MappingConfig>({
  dateColumn: '',
  amountColumn: '',
  descriptionColumn: '',
  dateFormat: 'MM/dd/yyyy',
  amountSign: 'negative-is-expense',
  defaultCategory: 'uncategorized',
})

const headerItems = computed(() => headers.value.map((h) => ({ label: h, value: h })))

const parsed = computed(() =>
  step.value === 3 ? applyMapping(rawRows.value, config.value) : [],
)
const dupes = computed(() =>
  accountId.value == null
    ? []
    : detectDuplicates(
        parsed.value,
        txnStore.page.rows.map((r) => ({
          accountId: r.accountId, date: r.date, amount: r.amount, description: r.description,
        })),
        accountId.value,
      ),
)
const include = ref<boolean[]>([])

onMounted(async () => {
  await accountsStore.load()
  savedMappings.value = await mappingApi.listImportMappings()
})

async function onFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  const text = await file.text()
  const result = parseCsv(text)
  headers.value = result.headers
  rawRows.value = result.rows
  step.value = 2
}

function applySavedMapping(m: ImportMapping) {
  config.value = { ...config.value, ...JSON.parse(m.config) }
}

function goToPreview() {
  // default-uncheck duplicates
  step.value = 3
  include.value = parsed.value.map((_, i) => !dupes.value[i])
}

async function saveMapping() {
  if (!newMappingName.value) return
  await mappingApi.createImportMapping({
    name: newMappingName.value,
    config: JSON.stringify(config.value),
    createdAt: DateTime.now().toISO()!,
  })
  savedMappings.value = await mappingApi.listImportMappings()
  newMappingName.value = ''
}

async function confirmImport() {
  if (accountId.value == null) return
  const now = DateTime.now().toISO()!
  const toInsert = parsed.value
    .filter((_, i) => include.value[i])
    .map((p) => ({
      accountId: accountId.value!,
      transferAccountId: null,
      amount: p.amount,
      description: p.description,
      date: p.date,
      type: p.type,
      category: p.category,
      isContribution: false,
      importSource: 'csv',
      updateBalance: false,
      createdAt: now,
    }))
  await bulkCreateTransactions(toInsert)
  await txnStore.load()
  emit('done')
}
</script>

<template>
  <div class="space-y-4">
    <!-- Step 1: file + account -->
    <div v-if="step === 1" class="space-y-3">
      <USelect
        v-model="accountId"
        :items="accountsStore.accounts.map((a) => ({ label: a.name, value: a.id }))"
        placeholder="Destination account"
      />
      <input type="file" accept=".csv" :disabled="accountId == null" @change="onFile" />
      <div v-if="savedMappings.length" class="text-sm">
        <p class="text-muted mb-1">Saved mappings:</p>
        <UButton v-for="m in savedMappings" :key="m.id" size="xs" variant="soft"
          class="mr-1" @click="applySavedMapping(m)">{{ m.name }}</UButton>
      </div>
    </div>

    <!-- Step 2: map columns -->
    <div v-else-if="step === 2" class="space-y-3">
      <USelect v-model="config.dateColumn" :items="headerItems" placeholder="Date column" />
      <USelect v-model="config.amountColumn" :items="headerItems" placeholder="Amount column" />
      <USelect v-model="config.descriptionColumn" :items="headerItems" placeholder="Description column" />
      <UInput v-model="config.dateFormat" placeholder="Date format (e.g. MM/dd/yyyy)" />
      <USelect
        v-model="config.amountSign"
        :items="[
          { label: 'Negative amounts are expenses', value: 'negative-is-expense' },
          { label: 'Positive amounts are expenses', value: 'positive-is-expense' },
        ]"
      />
      <div class="flex gap-2 items-center">
        <UInput v-model="newMappingName" placeholder="Save this mapping as…" class="w-52" />
        <UButton size="sm" variant="soft" :disabled="!newMappingName" @click="saveMapping">Save mapping</UButton>
      </div>
      <div class="flex justify-end">
        <UButton :disabled="!config.dateColumn || !config.amountColumn" @click="goToPreview">Preview</UButton>
      </div>
    </div>

    <!-- Step 3: preview + dedup -->
    <div v-else class="space-y-3">
      <p class="text-sm text-muted">
        {{ include.filter(Boolean).length }} of {{ parsed.length }} rows selected
        ({{ dupes.filter(Boolean).length }} possible duplicates unchecked).
      </p>
      <table class="w-full text-sm">
        <thead class="text-left text-muted border-b border-default">
          <tr><th></th><th>Date</th><th>Description</th><th>Type</th><th class="text-right">Amount</th></tr>
        </thead>
        <tbody>
          <tr v-for="(p, i) in parsed" :key="i" class="border-b border-default/50"
            :class="{ 'opacity-50': dupes[i] }">
            <td><UCheckbox v-model="include[i]" /></td>
            <td>{{ p.date }}</td>
            <td>{{ p.description }} <span v-if="dupes[i]" class="text-xs text-amber-600">(dup)</span></td>
            <td>{{ p.type }}</td>
            <td class="text-right tabular-nums">{{ p.amount }}</td>
          </tr>
        </tbody>
      </table>
      <div class="flex justify-end">
        <UButton :disabled="!include.some(Boolean)" @click="confirmImport">Import selected</UButton>
      </div>
    </div>
  </div>
</template>
