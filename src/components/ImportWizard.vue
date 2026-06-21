<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { DateTime } from 'luxon'
import { parseCsv } from '../lib/csv/parse'
import { applyMapping, autoDetectMapping, detectDuplicates, detectTransferCounterparts, parseAmount, type ExistingRef, type MappingConfig } from '../lib/csv/mapping'
import { bulkCreateTransactions, bulkCreateTransactionsWithSnapshots, listTransactions } from '../lib/api/transactions'
import * as mappingApi from '../lib/api/importMappings'
import * as categoryRulesApi from '../lib/api/categoryRules'
import { useToast } from '@nuxt/ui/composables'
import { addBalance } from '../lib/api/accounts'
import { useAccountsStore } from '../stores/accounts'
import { useTransactionsStore } from '../stores/transactions'
import { isLiability, isInvestment } from '../lib/accountTypes'
import { projectRunningBalances } from '../lib/csv/balanceProjection'
import { categoryItems } from '../lib/transactions/constants'
import type { ImportMapping } from '../lib/types/ImportMapping'
import type { CategoryRule } from '../lib/types/CategoryRule'
import { confirm } from '@tauri-apps/plugin-dialog'

const emit = defineEmits<{ done: [] }>()
const accountsStore = useAccountsStore()
const txnStore = useTransactionsStore()
const toast = useToast()

const step = ref<1 | 2 | 3>(1)
const accountId = ref<number | undefined>(undefined)
const headers = ref<string[]>([])
const rawRows = ref<Record<string, string>[]>([])
const savedMappings = ref<ImportMapping[]>([])
const categoryRules = ref<CategoryRule[]>([])
const newMappingName = ref('')
const appliedMappingId = ref<number | null>(null)
let appliedTimer: ReturnType<typeof setTimeout> | null = null

const editingMappingId = ref<number | null>(null)
const editingMappingName = ref('')

const config = ref<MappingConfig>({
  dateColumn: '',
  descriptionColumn: '',
  dateFormat: 'MM/dd/yyyy',
  amountMode: 'single',
  amountColumn: '',
  amountSign: 'negative-is-expense',
  creditColumn: '',
  debitColumn: '',
  invertSplit: false,
  defaultCategory: 'uncategorized',
  transferRules: [],
})

const headerItems = computed(() => headers.value.map((h) => ({ label: h, value: h })))

const isLiabilityAccount = computed(() => {
  if (accountId.value == null) return false
  const account = accountsStore.accounts.find((a) => a.id === accountId.value)
  return account ? isLiability(account.type) : false
})

const isInvestmentAccount = computed(() => {
  if (accountId.value == null) return false
  const acct = accountsStore.accounts.find((a) => a.id === accountId.value)
  return acct ? isInvestment(acct.type) : false
})

const allParsedRows = computed(() =>
  rawRows.value.length > 0 && config.value.dateColumn
    ? applyMapping(rawRows.value, config.value, categoryRules.value)
    : [],
)

const parsed = computed(() => (step.value === 3 ? allParsedRows.value : []))

const earliestDate = computed(() => {
  const dates = allParsedRows.value.map((r) => r.date).filter(Boolean)
  return dates.length ? dates.reduce((min, d) => (d < min ? d : min)) : ''
})

const priorSnapshot = computed(() => {
  if (!accountId.value || !earliestDate.value || !generateSnapshots.value) return null
  const candidates = accountsStore.allBalances.filter(
    (b) => b.accountId === accountId.value && b.recordedAt <= earliestDate.value,
  )
  if (!candidates.length) return null
  return candidates.reduce((latest, b) =>
    b.recordedAt > latest.recordedAt || (b.recordedAt === latest.recordedAt && b.id > latest.id)
      ? b
      : latest,
  )
})

// used in template (Task 3)
const needsSeed = computed(() => generateSnapshots.value && priorSnapshot.value === null && earliestDate.value !== '')

const baseBalance = computed(() => priorSnapshot.value?.balance ?? seedBalance.value)

// Rows that count toward the running balance: imported rows + balance-only (detected
// duplicates that already exist in the DB and are already affecting the real balance).
const includeInBalance = computed(() =>
  allParsedRows.value.map((_, i) => {
    const s = include.value[i]
    return s === 'import' || s === 'balance-only'
  }),
)

const runningBalances = computed(() =>
  // Balances are positive magnitudes; for a liability the projection inverts the
  // income/expense sign (a purchase raises debt, a refund lowers it) and a payment
  // (transfer) lowers it. The backend applies the identical liability-aware sign on
  // save, so the preview and the saved snapshots agree.
  generateSnapshots.value
    ? projectRunningBalances(
        allParsedRows.value,
        includeInBalance.value,
        baseBalance.value,
        isLiabilityAccount.value,
      )
    : [],
)

const existingRefs = ref<ExistingRef[]>([])

const exactDupes = computed(() =>
  accountId.value == null
    ? []
    : detectDuplicates(parsed.value, existingRefs.value, accountId.value),
)

const transferDupes = computed(() =>
  accountId.value == null
    ? []
    : detectTransferCounterparts(parsed.value, existingRefs.value, accountId.value),
)

const dupes = computed(() =>
  parsed.value.map((_, i) => Boolean(exactDupes.value[i]) || Boolean(transferDupes.value[i])),
)

function rowClass(row: { index: number }) {
  return include.value[row.index] !== 'import' ? 'opacity-50' : ''
}

const canPreview = computed(() => {
  if (!config.value.dateColumn) return false
  if (config.value.amountMode === 'single') return !!config.value.amountColumn
  return !!config.value.creditColumn && !!config.value.debitColumn
})

const exampleEntry = computed(() => {
  if (rawRows.value.length === 0) return null
  const cfg = config.value

  let row: Record<string, string> | undefined
  if (cfg.amountMode === 'single') {
    if (!cfg.amountColumn) return null
    row = rawRows.value.find((r) => parseAmount(r[cfg.amountColumn]) !== 0)
  } else {
    if (!cfg.creditColumn || !cfg.debitColumn) return null
    row = rawRows.value.find((r) => {
      const c = cfg.creditColumn ? parseAmount(r[cfg.creditColumn]) : 0
      const d = cfg.debitColumn ? parseAmount(r[cfg.debitColumn]) : 0
      return c !== 0 || d !== 0
    })
  }

  if (!row) return null
  return {
    raw: row,
    parsed: applyMapping([row], cfg)[0],
  }
})

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

const csvFile = ref<File | null>(null)
const include = ref<('import' | 'balance-only' | 'skip')[]>([])
const rowCategories = ref<string[]>([])
const manuallyOverridden = ref<boolean[]>([])
const newRuleKeyword = ref('')
const newRuleCategory = ref('discretionary')
const newTransferKeyword = ref('')
const newTransferAccountId = ref<number | undefined>(undefined)
const generateSnapshots = ref(false)
const importing = ref(false)
const seedBalance = ref(0)

const previewColumns = computed(() => [
  { id: 'include', header: '' },
  { accessorKey: 'date', header: 'Date' },
  { accessorKey: 'description', header: 'Description', meta: { class: { th: 'w-full', td: 'max-w-0 w-full' } } },
  { id: 'type', header: 'Type' },
  { id: 'category', header: 'Category' },
  { id: 'amount', header: 'Amount', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  ...(generateSnapshots.value
    ? [{ id: 'balance', header: 'Balance', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } }]
    : []),
])

onMounted(async () => {
  await accountsStore.load()
  savedMappings.value = await mappingApi.listImportMappings()
  categoryRules.value = await categoryRulesApi.listCategoryRules()
})

watch(csvFile, async (file) => {
  if (!file) return
  const text = await file.text()
  const result = parseCsv(text)
  headers.value = result.headers
  rawRows.value = result.rows
  const detected = autoDetectMapping(result.headers, result.rows)
  config.value = { ...config.value, ...detected }
  step.value = 2
})

function applySavedMapping(m: ImportMapping) {
  cancelRename()
  config.value = { ...config.value, ...JSON.parse(m.config) }
  if (appliedTimer !== null) clearTimeout(appliedTimer)
  appliedMappingId.value = m.id
  appliedTimer = setTimeout(() => {
    appliedMappingId.value = null
    appliedTimer = null
  }, 1750)
}

function startRename(m: ImportMapping) {
  editingMappingId.value = m.id
  editingMappingName.value = m.name
}

function cancelRename() {
  editingMappingId.value = null
  editingMappingName.value = ''
}

async function saveRename(m: ImportMapping) {
  const trimmed = editingMappingName.value.trim()
  if (!trimmed) {
    cancelRename()
    return
  }
  try {
    await mappingApi.updateImportMapping(m.id, trimmed)
    savedMappings.value = await mappingApi.listImportMappings()
  } catch {
    toast.add({ title: 'Failed to rename mapping', color: 'error' })
  }
  cancelRename()
}

async function deleteMapping(m: ImportMapping) {
  const ok = await confirm(`Delete "${m.name}"?`, { title: 'Delete mapping', kind: 'warning' })
  if (!ok) return
  try {
    await mappingApi.deleteImportMapping(m.id)
    savedMappings.value = await mappingApi.listImportMappings()
  } catch {
    toast.add({ title: 'Failed to delete mapping', color: 'error' })
  }
}

function goToPreview() {
  step.value = 3
  include.value = parsed.value.map((_, i) => (dupes.value[i] ? 'balance-only' : 'import'))
  rowCategories.value = parsed.value.map((p) => p.category)
  manuallyOverridden.value = parsed.value.map(() => false)
}

function toggleInclude(i: number) {
  const s = include.value[i]
  if (s === 'import') include.value[i] = 'balance-only'
  else if (s === 'balance-only') include.value[i] = 'skip'
  else include.value[i] = 'import'
}

watch(parsed, (newParsed) => {
  if (step.value !== 3) return
  rowCategories.value = newParsed.map((p, i) =>
    manuallyOverridden.value[i] ? rowCategories.value[i] : p.category,
  )
})

watch(accountId, async (newId) => {
  if (newId == null) {
    existingRefs.value = []
    return
  }
  const acct = accountsStore.accounts.find((a) => a.id === newId)
  if (acct && isInvestment(acct.type)) generateSnapshots.value = false

  // The backend filter is `(account_id = ? OR transfer_account_id = ?)`, so this
  // returns both this account's own rows (for exact dedup) and transfers whose
  // other side is this account (for transfer-counterpart dedup).
  const page = await listTransactions({ accountId: newId, limit: 1_000_000 })
  existingRefs.value = page.rows.map((r) => ({
    accountId: r.accountId,
    date: r.date,
    amount: r.amount,
    description: r.description,
    type: r.type,
    transferAccountId: r.transferAccountId,
  }))
})

watch(priorSnapshot, (snap) => {
  if (snap !== null) seedBalance.value = 0
})

async function saveQuickRule() {
  if (!newRuleKeyword.value.trim()) return
  await categoryRulesApi.createCategoryRule(
    newRuleKeyword.value.trim().toLowerCase(),
    newRuleCategory.value,
    DateTime.now().toISO()!,
  )
  categoryRules.value = await categoryRulesApi.listCategoryRules()
  newRuleKeyword.value = ''
  newRuleCategory.value = 'discretionary'
}

function addTransferRule() {
  if (!newTransferKeyword.value.trim() || newTransferAccountId.value == null) return
  config.value.transferRules.push({
    keyword: newTransferKeyword.value.trim().toLowerCase(),
    transferAccountId: newTransferAccountId.value,
  })
  newTransferKeyword.value = ''
  newTransferAccountId.value = undefined
}

function removeTransferRule(index: number) {
  config.value.transferRules.splice(index, 1)
}

function accountName(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? 'Unknown account'
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

  const selectedRows = parsed.value
    .map((p, i) => ({ p, i }))
    .filter(({ i }) => include.value[i] === 'import')
    .map(({ p, i }) => ({
      accountId: accountId.value!,
      transferAccountId: p.transferAccountId ?? null,
      amount: p.amount,
      description: p.description,
      date: p.date,
      type: p.type,
      category: p.type === 'transfer' ? 'uncategorized' : (rowCategories.value[i] ?? p.category),
      isContribution: false,
      importSource: 'csv',
      createdAt: now,
    }))

  if (!generateSnapshots.value) {
    await bulkCreateTransactions(
      selectedRows.map((r) => {
        const isLiabTransfer = isLiabilityAccount.value && r.type === 'transfer' && r.transferAccountId != null
        return {
          ...r,
          accountId: isLiabTransfer ? r.transferAccountId! : r.accountId,
          transferAccountId: isLiabTransfer ? r.accountId : r.transferAccountId,
          updateBalance: false,
        }
      }),
    )
  } else {
    importing.value = true
    try {
      if (needsSeed.value) {
        await addBalance({
          accountId: accountId.value!,
          balance: seedBalance.value,
          recordedAt: earliestDate.value,
        })
      }
      const sorted = [...selectedRows]
        .sort((a, b) => a.date.localeCompare(b.date))
        .map((row) => {
          const isLiabTransfer =
            isLiabilityAccount.value && row.type === 'transfer' && row.transferAccountId != null
          return {
            ...row,
            accountId: isLiabTransfer ? row.transferAccountId! : row.accountId,
            transferAccountId: isLiabTransfer ? row.accountId : row.transferAccountId,
            updateBalance: true,
          }
        })
      await bulkCreateTransactionsWithSnapshots(sorted)
      await accountsStore.load()
    } catch (e: any) {
      toast.add({
        title: 'Import failed',
        description: e?.message ?? String(e),
        color: 'error',
        duration: 0,
      })
      return
    } finally {
      importing.value = false
    }
  }

  await txnStore.load()
  emit('done')
}
</script>

<template>
  <div class="space-y-4">
    <!-- Step 1: file + account -->
    <div v-if="step === 1" class="space-y-5">
      <div class="space-y-1.5">
        <p class="text-sm font-medium">Destination account</p>
        <USelect
          v-model="accountId"
          :items="accountsStore.accounts.map((a) => ({ label: a.name, value: a.id }))"
          placeholder="Select an account"
          class="w-full"
        />
      </div>
      <UFileUpload
        v-model="csvFile"
        accept=".csv"
        label="Drop your CSV file here"
        description="or click to browse"
        class="w-full"
      />
    </div>

    <!-- Step 2: map columns -->
    <div v-else-if="step === 2" class="space-y-5">

      <!-- LOAD SAVED MAPPING -->
      <div v-if="savedMappings.length" class="space-y-1.5">
        <div class="mb-3">
          <p class="text-xs">Apply a saved column mapping</p>
          <p class="text-xs text-muted">Click to load, right-click to rename or delete it.</p>
        </div>
        <div class="flex items-center gap-1.5 flex-wrap">
          <template v-for="m in savedMappings" :key="m.id">
            <UContextMenu
              v-if="editingMappingId !== m.id"
              :items="[[{ label: 'Rename', icon: 'i-heroicons-pencil', onSelect: () => startRename(m) }, { label: 'Delete', icon: 'i-heroicons-trash', color: 'error', onSelect: () => deleteMapping(m) }]]"
            >
              <UButton
                size="xs"
                variant="soft"
                :color="appliedMappingId === m.id ? 'success' : 'neutral'"
                :leading-icon="appliedMappingId === m.id ? 'i-heroicons-check' : undefined"
                @click="applySavedMapping(m)"
              >{{ appliedMappingId === m.id ? 'Applied' : m.name }}</UButton>
            </UContextMenu>
            <div v-else class="flex items-center gap-0.5">
              <UInput v-model="editingMappingName" size="xs" class="w-28" @keydown.enter="saveRename(m)" @keydown.escape="cancelRename" />
              <UButton size="xs" variant="ghost" color="success" icon="i-heroicons-check" aria-label="Save rename" @click="saveRename(m)" />
              <UButton size="xs" variant="ghost" color="neutral" icon="i-heroicons-x-mark" aria-label="Cancel rename" @click="cancelRename" />
            </div>
          </template>
        </div>
      </div>

      <div class="flex gap-5">
        <!-- LEFT: core config -->
        <div class="flex-1 space-y-5">

          <!-- COLUMN MAPPING -->
          <div class="space-y-3">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Column Mapping</p>
            <div class="flex gap-3">
              <div class="flex-1">
                <p class="text-xs text-muted mb-1">Date column</p>
                <USelect v-model="config.dateColumn" :items="headerItems" placeholder="Select column" class="w-full" />
              </div>
              <div class="w-36">
                <p class="text-xs text-muted mb-1">Date format</p>
                <UInput v-model="config.dateFormat" placeholder="MM/dd/yyyy" class="w-full" />
              </div>
            </div>
            <div>
              <p class="text-xs text-muted mb-1">Description column (optional)</p>
              <USelect v-model="config.descriptionColumn" :items="headerItems" placeholder="Select column" class="w-full" />
            </div>
          </div>

          <!-- AMOUNT -->
          <div class="space-y-3">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Amount</p>

            <div class="flex gap-1 p-1 rounded-lg bg-muted">
              <UButton
                type="button"
                :variant="config.amountMode === 'single' ? 'solid' : 'ghost'"
                size="sm"
                class="flex-1"
                @click="config.amountMode = 'single'"
              >Single column</UButton>
              <UButton
                type="button"
                :variant="config.amountMode === 'split' ? 'solid' : 'ghost'"
                size="sm"
                class="flex-1"
                @click="config.amountMode = 'split'"
              >Credit + Debit columns</UButton>
            </div>

            <template v-if="config.amountMode === 'single'">
              <div>
                <p class="text-xs text-muted mb-1">Amount column</p>
                <USelect v-model="config.amountColumn" :items="headerItems" placeholder="Select column" class="w-full" />
              </div>
              <div>
                <p class="text-xs text-muted mb-1">Amount sign</p>
                <USelect
                  v-model="config.amountSign"
                  :items="[
                    { label: 'Negative amounts are expenses', value: 'negative-is-expense' },
                    { label: 'Positive amounts are expenses', value: 'positive-is-expense' },
                  ]"
                  class="w-full"
                />
              </div>
            </template>

            <template v-else>
              <div class="flex gap-3">
                <div class="flex-1">
                  <p class="text-xs text-muted mb-1">Credit column</p>
                  <USelect v-model="config.creditColumn" :items="headerItems" placeholder="Select column" class="w-full" />
                </div>
                <div class="flex-1">
                  <p class="text-xs text-muted mb-1">Debit column</p>
                  <USelect v-model="config.debitColumn" :items="headerItems" placeholder="Select column" class="w-full" />
                </div>
              </div>
              <USwitch v-model="config.invertSplit" label="Invert credit/debit direction" />
            </template>

            <!-- Live example card -->
            <div class="rounded-lg border border-default p-3 text-sm space-y-1.5">
              <p class="text-xs text-muted">Example from your CSV</p>
              <template v-if="exampleEntry">
                <div class="flex items-center gap-2">
                  <span
                    class="text-xs font-medium px-2 py-0.5 rounded-full"
                    :class="exampleEntry.parsed.type === 'income'
                      ? 'bg-success/15 text-success'
                      : 'bg-error/15 text-error'"
                  >{{ exampleEntry.parsed.type }}</span>
                  <span class="tabular-nums font-medium">{{ money(exampleEntry.parsed.amount) }}</span>
                </div>
                <p class="text-xs text-muted">
                  {{ exampleEntry.parsed.date }} · {{ exampleEntry.parsed.description || '—' }}
                  <template v-if="config.amountMode === 'split'">
                    · Credit: {{ exampleEntry.raw[config.creditColumn] || '—' }}
                    / Debit: {{ exampleEntry.raw[config.debitColumn] || '—' }}
                  </template>
                  <template v-else>
                    · Raw: {{ exampleEntry.raw[config.amountColumn] || '—' }}
                  </template>
                </p>
              </template>
              <p v-else class="text-xs text-muted">Select columns to see an example.</p>
            </div>
          </div>

          <!-- CATEGORY DEFAULTS -->
          <div class="space-y-3">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Category Defaults</p>
            <div>
              <p class="text-xs text-muted mb-1">Default category for unmatched rows</p>
              <USelect v-model="config.defaultCategory" :items="categoryItems" class="w-full" />
            </div>
          </div>

        </div>

        <!-- RIGHT: transfer rules + save mapping -->
        <div class="w-72 bg-muted border border-default/50 rounded-xl space-y-5 -my-1 p-4">

          <!-- TRANSFER RULES -->
          <div class="space-y-2">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Transfer Rules</p>
            <p class="text-xs text-muted">Mark rows matching a keyword as a transfer between this account and another.</p>

            <div v-if="config.transferRules.length" class="space-y-1">
              <div
                v-for="(rule, i) in config.transferRules"
                :key="i"
                class="flex items-center gap-2 text-sm"
              >
                <span class="flex-1 truncate">{{ rule.keyword }}</span>
                <span class="text-muted">↔</span>
                <span class="flex-1 truncate">{{ accountName(rule.transferAccountId) }}</span>
                <UButton size="xs" variant="ghost" color="red" @click="removeTransferRule(i)">Remove</UButton>
              </div>
            </div>

            <div class="space-y-2">
              <UInput
                v-model="newTransferKeyword"
                placeholder="keyword (e.g. payment thank you)"
                size="xs"
                class="w-full"
              />
              <div class="flex gap-2 items-center">
                <USelect
                  v-model="newTransferAccountId"
                  :items="accountsStore.accounts.map((a) => ({ label: a.name, value: a.id }))"
                  placeholder="Other account"
                  size="xs"
                  class="flex-1 min-w-0"
                />
                <UButton
                  size="xs"
                  variant="soft"
                  :disabled="!newTransferKeyword.trim() || newTransferAccountId == null"
                  @click="addTransferRule"
                >Add rule</UButton>
              </div>
            </div>
          </div>

          <!-- SAVE MAPPING -->
          <div class="space-y-2">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Save Mapping</p>
            <div class="flex gap-2 items-center">
              <UInput v-model="newMappingName" placeholder="Save this mapping as…" size="xs" class="flex-1 min-w-0" />
              <UButton size="xs" variant="soft" :disabled="!newMappingName" @click="saveMapping">Save</UButton>
            </div>
          </div>

          <!-- GENERATE BALANCE SNAPSHOTS -->
          <div v-if="!isInvestmentAccount && rawRows.length > 0" class="space-y-2">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Balance Snapshots</p>
            <USwitch v-model="generateSnapshots" label="Generate balance snapshots" />
            <template v-if="generateSnapshots">
              <div v-if="priorSnapshot" class="text-xs text-muted">
                Will cascade from your
                {{ DateTime.fromISO(priorSnapshot.recordedAt).toLocaleString(DateTime.DATE_MED) }} snapshot of {{ money(priorSnapshot.balance) }}.
              </div>
              <div v-else-if="needsSeed" class="space-y-1">
                <p class="text-xs text-muted">
                  No balance found before {{ DateTime.fromISO(earliestDate).toLocaleString(DateTime.DATE_MED) }}. Enter a starting balance:
                </p>
                <UInput
                  v-model.number="seedBalance"
                  type="number"
                  size="xs"
                  class="w-full"
                  placeholder="0.00"
                />
              </div>
            </template>
          </div>

        </div>
      </div>

      <div class="flex justify-between pt-2">
        <UButton variant="ghost" @click="step = 1">← Back</UButton>
        <UButton :disabled="!canPreview" @click="goToPreview">Preview</UButton>
      </div>

    </div>

    <!-- Step 3: preview + dedup -->
    <div v-else class="space-y-3">
      <p class="text-sm text-muted">
        {{ include.filter(s => s === 'import').length }} of {{ parsed.length }} rows selected for import<template v-if="include.some(s => s === 'balance-only')"> · {{ include.filter(s => s === 'balance-only').length }} counted in balance only</template>.
      </p>
      <UTable
        :data="parsed"
        :columns="previewColumns"
        :meta="{ class: { tr: rowClass } }"
      >
        <template #include-cell="{ row }">
          <UTooltip :text="include[row.index] === 'balance-only' ? 'Counted in balance, will not be imported (duplicate)' : undefined">
            <UCheckbox
              :model-value="include[row.index] === 'import' ? true : include[row.index] === 'balance-only' ? 'indeterminate' : false"
              @update:model-value="toggleInclude(row.index)"
            />
          </UTooltip>
        </template>
        <template #description-cell="{ row }">
          <div class="flex items-center gap-1.5 min-w-0">
            <UTooltip :text="row.original.description" class="min-w-0">
              <span class="block truncate">{{ row.original.description }}</span>
            </UTooltip>
            <span v-if="exactDupes[row.index]" class="text-xs text-amber-600 shrink-0">(dup)</span>
            <UTooltip
              v-else-if="transferDupes[row.index]"
              text="Looks like the other side of an existing transfer — unchecked to avoid double-counting."
            >
              <span class="text-xs text-amber-600 shrink-0">(transfer dup)</span>
            </UTooltip>
          </div>
        </template>
        <template #type-cell="{ row }">
          <template v-if="row.original.type === 'transfer'">
            <!-- Liability transfers (e.g. card payments) flow INTO this account from
                 the other side, so show the other account as the source. -->
            <template v-if="isLiabilityAccount">
              {{ accountName(row.original.transferAccountId!) }} → transfer
            </template>
            <template v-else>
              transfer → {{ accountName(row.original.transferAccountId!) }}
            </template>
          </template>
          <template v-else>{{ row.original.type }}</template>
        </template>
        <template #category-cell="{ row }">
          <USelect
            v-if="row.original.type !== 'transfer'"
            v-model="rowCategories[row.index]"
            :items="categoryItems"
            size="xs"
            class="w-36"
            @update:model-value="manuallyOverridden[row.index] = true"
          />
          <span v-else class="text-xs text-muted">—</span>
        </template>
        <template #amount-cell="{ row }">{{ row.original.amount }}</template>
        <template #balance-cell="{ row }">
          <span v-if="runningBalances[row.index] != null">
            {{ money(runningBalances[row.index]!) }}
          </span>
          <span v-else class="text-muted">—</span>
        </template>
      </UTable>
      <div class="flex gap-2 items-center pt-1 border-t border-default">
        <p class="text-xs text-muted shrink-0">Add rule:</p>
        <UInput
          v-model="newRuleKeyword"
          placeholder="keyword"
          size="xs"
          class="flex-1"
          @keydown.enter="saveQuickRule"
        />
        <USelect v-model="newRuleCategory" :items="categoryItems" size="xs" class="w-36" />
        <UButton size="xs" variant="soft" :disabled="!newRuleKeyword.trim()" @click="saveQuickRule">
          Save rule
        </UButton>
      </div>
      <div class="flex justify-between">
        <UButton variant="ghost" :disabled="importing" @click="step = 2">← Back to settings</UButton>
        <div class="flex items-center gap-3">
          <p v-if="importing" class="text-sm text-muted">
            Saving {{ include.filter(s => s === 'import').length }} transactions…
          </p>
          <UButton :disabled="!include.some(s => s === 'import')" :loading="importing" @click="confirmImport">
            Import selected
          </UButton>
        </div>
      </div>
    </div>
  </div>
</template>
