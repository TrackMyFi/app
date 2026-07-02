<script setup lang="ts">
import { reactive, ref, computed, onMounted } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { confirm } from '@tauri-apps/plugin-dialog'
import { DateTime } from 'luxon'
import { useFireProfileStore } from '../stores/fireProfile'
import { useSyncStore } from '../stores/sync'
import { useStorageStore } from '../stores/storage'
import { useAccountsStore } from '../stores/accounts'
import { useUpdaterStore, CHECK_INTERVAL_MS } from '../stores/updater'
import {
  saveSyncConfig,
  clearSyncConfig,
  syncNow,
  restartApp,
} from '../lib/api/sync'
import * as categoryRulesApi from '../lib/api/categoryRules'
import * as vendorRulesApi from '../lib/api/vendorRules'
import * as transactionsApi from '../lib/api/transactions'
import { countMigratableAttachments, migrateAndSaveStorageConfig } from '../lib/api/storage'
import type { FireProfile } from '../lib/types/FireProfile'
import type { CategoryRule } from '../lib/types/CategoryRule'
import type { VendorRule } from '../lib/types/VendorRule'
import type { Transaction } from '../lib/types/Transaction'
import type { VendorRuleInput } from '../lib/expenses/merchants'
import { suggestVendorRules, type VendorRuleSuggestion } from '../lib/expenses/vendorSuggestions'
import DeleteDataModal from '../components/DeleteDataModal.vue'
import CurrencyInput from '../components/CurrencyInput.vue'
import PercentInput from '../components/PercentInput.vue'
import DateInput from '../components/DateInput.vue'
import { categoryItems, labelForCategory } from '../lib/transactions/constants'
import PageError from '../components/PageError.vue'
import { usePageData } from '../composables/usePageData'

interface FireProfileForm {
  dateOfBirth: string | null
  targetRetirementAge: number
  annualExpensesTarget: number
  leanFireAnnualExpenses: number | null
  fatFireAnnualExpenses: number | null
  annualIncome: number
  expectedReturnRate: number
  inflationRate: number
  hsaCoverage: string
}

const store = useFireProfileStore()
const toast = useToast()
const { error, run, retry } = usePageData()
const form = reactive<FireProfileForm>({
  dateOfBirth: null,
  targetRetirementAge: 0,
  annualExpensesTarget: 0,
  leanFireAnnualExpenses: null,
  fatFireAnnualExpenses: null,
  annualIncome: 0,
  expectedReturnRate: 0,
  inflationRate: 0,
  hsaCoverage: 'self',
})

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

onMounted(() => run(async () => {
  await store.load()
  if (store.profile) Object.assign(form, store.profile)
  categoryRules.value = await categoryRulesApi.listCategoryRules()
  vendorRules.value = await vendorRulesApi.listVendorRules()
  await accountsStore.load()
  suggestionTransactions.value = (await transactionsApi.listTransactions({ limit: null })).rows
  await updater.loadVersion()
  await storageStore.load()
  if (storageStore.config) {
    storageProvider.value = storageStore.config.provider || 'local'
    storageBucket.value = storageStore.config.bucketName ?? ''
    storageR2AccountId.value = storageStore.config.r2AccountId ?? ''
    storageS3Region.value = storageStore.config.s3Region ?? ''
  }
}))

async function onSubmit() {
  const profile: FireProfile = {
    dateOfBirth: form.dateOfBirth || null,
    targetRetirementAge: form.targetRetirementAge,
    annualExpensesTarget: form.annualExpensesTarget ?? 0,
    leanFireAnnualExpenses: form.leanFireAnnualExpenses,
    fatFireAnnualExpenses: form.fatFireAnnualExpenses,
    annualIncome: form.annualIncome ?? 0,
    expectedReturnRate: form.expectedReturnRate ?? 0,
    inflationRate: form.inflationRate ?? 0,
    hsaCoverage: form.hsaCoverage,
    onboardingCompleted: store.profile?.onboardingCompleted ?? false,
  }
  savingProfile.value = true
  try {
    await store.save(profile)
    toast.add({ title: 'Profile updated', color: 'success' })
  } catch (err) {
    toast.add({ title: 'Failed to save profile', description: String(err), color: 'error' })
  } finally {
    savingProfile.value = false
  }
}

const savingProfile = ref(false)

const accountsStore = useAccountsStore()

const syncStore = useSyncStore()
const syncUrl = ref('')
const syncToken = ref('')
const syncBusy = ref(false)
const syncMessage = ref('')

// ---- Attachment Storage ----
const storageStore = useStorageStore()
const storageProvider = ref('local')
const storageBucket = ref('')
const storageR2AccountId = ref('')
const storageS3Region = ref('')
const storageAccessKey = ref('')
const storageSecretKey = ref('')
const storageServiceAccountJson = ref('')
const storageBusy = ref(false)
const storageMessage = ref('')

const PROVIDER_OPTIONS = [
  { label: 'Local storage (no sync)', value: 'local' },
  { label: 'Cloudflare R2', value: 'r2' },
  { label: 'Google Cloud Storage', value: 'gcs' },
  { label: 'Amazon S3', value: 's3' },
]

function providerLabel(value: string) {
  return PROVIDER_OPTIONS.find((o) => o.value === value)?.label ?? value
}

const showDeleteModal = ref(false)

const categoryRules = ref<CategoryRule[]>([])
const newRuleKeyword = ref('')
const newRuleCategory = ref('discretionary')
const savingRule = ref(false)
const removingRuleId = ref<number | null>(null)

const vendorRules = ref<VendorRule[]>([])
const newVendorKeyword = ref('')
const newVendorName = ref('')
const savingVendorRule = ref(false)
const removingVendorRuleId = ref<number | null>(null)

// ---- Vendor rule suggestions (from the user's own transaction history) ----
const suggestionTransactions = ref<Transaction[]>([])
const dismissedSuggestionKeys = ref<Set<string>>(new Set())
const addingSuggestionKey = ref<string | null>(null)

const vendorRuleInputs = computed<VendorRuleInput[]>(() =>
  vendorRules.value.map((r) => ({ keyword: r.keyword, vendorName: r.vendorName })),
)

const vendorRuleSuggestions = computed(() =>
  suggestVendorRules(suggestionTransactions.value, accountsStore.accounts, vendorRuleInputs.value)
    .filter((s) => !dismissedSuggestionKeys.value.has(s.key)),
)

async function acceptVendorRuleSuggestion(s: VendorRuleSuggestion) {
  addingSuggestionKey.value = s.key
  try {
    await vendorRulesApi.createVendorRule(s.keyword, s.vendorName, DateTime.now().toISO()!)
    vendorRules.value = await vendorRulesApi.listVendorRules()
  } catch (err) {
    toast.add({ title: 'Failed to add rule', description: String(err), color: 'error' })
  } finally {
    addingSuggestionKey.value = null
  }
}

function dismissVendorRuleSuggestion(s: VendorRuleSuggestion) {
  dismissedSuggestionKeys.value = new Set(dismissedSuggestionKeys.value).add(s.key)
}

const isSynced = computed(() => syncStore.status?.mode === 'synced')
const lastSynced = computed(() => {
  const ms = syncStore.status?.lastSyncedAt
  return ms ? new Date(ms).toLocaleString() : 'never'
})

async function enableSync() {
  if (!syncUrl.value || !syncToken.value) {
    syncMessage.value = 'Enter both the database URL and the auth token.'
    return
  }
  syncBusy.value = true
  syncMessage.value = ''
  try {
    const outcome = await saveSyncConfig(syncUrl.value.trim(), syncToken.value.trim())
    syncToken.value = ''
    const restart = await confirm(`${outcome}\n\nRestart now to start syncing?`, {
      title: 'Sync enabled',
      kind: 'info',
    })
    if (restart) await restartApp()
  } catch (e) {
    syncMessage.value = String(e)
  } finally {
    syncBusy.value = false
  }
}

async function disableSync() {
  const ok = await confirm(
    'Stop syncing on this device? Your data stays on this machine; the cloud copy is left untouched.',
    { title: 'Disable sync', kind: 'warning' },
  )
  if (!ok) return
  syncBusy.value = true
  try {
    await clearSyncConfig()
    const restart = await confirm('Sync disabled. Restart now to apply?', {
      title: 'Disable sync',
      kind: 'info',
    })
    if (restart) await restartApp()
  } catch (e) {
    syncMessage.value = String(e)
  } finally {
    syncBusy.value = false
  }
}

async function runSyncNow() {
  syncBusy.value = true
  try {
    syncMessage.value = ''
    await syncNow()
  } catch (e) {
    syncMessage.value = String(e)
  } finally {
    syncBusy.value = false
  }
}

async function addCategoryRule() {
  if (!newRuleKeyword.value.trim()) return
  savingRule.value = true
  try {
    await categoryRulesApi.createCategoryRule(
      newRuleKeyword.value.trim().toLowerCase(),
      newRuleCategory.value,
      DateTime.now().toISO()!,
    )
    categoryRules.value = await categoryRulesApi.listCategoryRules()
    newRuleKeyword.value = ''
    newRuleCategory.value = 'discretionary'
  } catch (err) {
    toast.add({ title: 'Failed to add rule', description: String(err), color: 'error' })
  } finally {
    savingRule.value = false
  }
}

async function removeCategoryRule(id: number) {
  removingRuleId.value = id
  try {
    await categoryRulesApi.deleteCategoryRule(id)
    categoryRules.value = await categoryRulesApi.listCategoryRules()
  } catch (err) {
    toast.add({ title: 'Failed to delete rule', description: String(err), color: 'error' })
  } finally {
    removingRuleId.value = null
  }
}

const ruleColumns = [
  { accessorKey: 'keyword', header: 'Keyword' },
  { accessorKey: 'category', header: 'Category', cell: ({ row }: { row: { original: { category: string } } }) => labelForCategory(row.original.category) },
  { id: 'actions', header: '', meta: { class: { td: 'text-right' } } },
]

async function addVendorRule() {
  if (!newVendorKeyword.value.trim() || !newVendorName.value.trim()) return
  savingVendorRule.value = true
  try {
    await vendorRulesApi.createVendorRule(
      newVendorKeyword.value.trim().toLowerCase(),
      newVendorName.value.trim(),
      DateTime.now().toISO()!,
    )
    vendorRules.value = await vendorRulesApi.listVendorRules()
    newVendorKeyword.value = ''
    newVendorName.value = ''
  } catch (err) {
    toast.add({ title: 'Failed to add rule', description: String(err), color: 'error' })
  } finally {
    savingVendorRule.value = false
  }
}

async function removeVendorRule(id: number) {
  removingVendorRuleId.value = id
  try {
    await vendorRulesApi.deleteVendorRule(id)
    vendorRules.value = await vendorRulesApi.listVendorRules()
  } catch (err) {
    toast.add({ title: 'Failed to delete rule', description: String(err), color: 'error' })
  } finally {
    removingVendorRuleId.value = null
  }
}

const vendorRuleColumns = [
  { accessorKey: 'keyword', header: 'Keyword' },
  { accessorKey: 'vendorName', header: 'Vendor name' },
  { id: 'actions', header: '', meta: { class: { td: 'text-right' } } },
]

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

async function saveStorageSettings() {
  storageBusy.value = true
  storageMessage.value = ''
  try {
    const newProvider = storageProvider.value
    const currentProvider = storageStore.config?.provider ?? 'local'
    const args = {
      provider: newProvider,
      bucketName: storageBucket.value.trim() || null,
      r2AccountId: storageR2AccountId.value.trim() || null,
      s3Region: storageS3Region.value.trim() || null,
      accessKeyId: storageAccessKey.value.trim() || null,
      secretAccessKey: storageSecretKey.value.trim() || null,
      serviceAccountJson: storageServiceAccountJson.value.trim() || null,
    }

    // When switching providers, check if any existing attachments need migrating.
    if (newProvider !== currentProvider) {
      const migratable = await countMigratableAttachments(newProvider)
      if (migratable > 0) {
        const fileWord = migratable === 1 ? 'attachment' : 'attachments'
        const shouldMigrate = await confirm(
          `You have ${migratable} ${fileWord} stored in ${providerLabel(currentProvider)}. ` +
            `Migrate ${migratable === 1 ? 'it' : 'them'} to ${providerLabel(newProvider)} now?\n\n` +
            `If you skip, those files won't be accessible from the new storage until migrated.`,
          { title: 'Migrate attachments?', kind: 'info' },
        )
        if (shouldMigrate) {
          const result = await migrateAndSaveStorageConfig(args)
          storageAccessKey.value = ''
          storageSecretKey.value = ''
          storageServiceAccountJson.value = ''
          storageMessage.value = ''
          await storageStore.load()
          const failNote =
            result.failed > 0
              ? ` (${result.failed} failed: ${result.failedNames.join(', ')})`
              : ''
          toast.add({
            title: `Migrated ${result.migrated} ${result.migrated === 1 ? 'attachment' : 'attachments'} to ${providerLabel(newProvider)}${failNote}`,
            color: result.failed > 0 ? 'warning' : 'success',
          })
          return
        }
      }
    }

    // No migration needed or user chose to skip — plain save.
    await storageStore.save(args)
    storageAccessKey.value = ''
    storageSecretKey.value = ''
    storageServiceAccountJson.value = ''
    storageMessage.value = ''
    toast.add({ title: 'Attachment storage saved', color: 'success' })
  } catch (e) {
    storageMessage.value = String(e)
  } finally {
    storageBusy.value = false
  }
}

async function clearStorageSettings() {
  const ok = await confirm(
    'Reset to local storage? Existing cloud-stored attachments will remain in the bucket but won\'t be accessible from this device until you reconnect.',
    { title: 'Clear storage config', kind: 'warning' },
  )
  if (!ok) return
  storageBusy.value = true
  try {
    await storageStore.clear()
    storageProvider.value = 'local'
    storageBucket.value = ''
    storageR2AccountId.value = ''
    storageS3Region.value = ''
    storageAccessKey.value = ''
    storageSecretKey.value = ''
    storageServiceAccountJson.value = ''
    toast.add({ title: 'Attachment storage reset to local', color: 'success' })
  } catch (e) {
    storageMessage.value = String(e)
  } finally {
    storageBusy.value = false
  }
}
</script>

<template>
  <div class="p-6 max-w-3xl space-y-8">
    <PageError v-if="error" :message="error" @retry="retry" />

    <section class="space-y-4">
      <h1 class="text-2xl font-bold">FIRE Profile</h1>
      <UForm :state="form" @submit="onSubmit" class="space-y-4">
        <div class="grid grid-cols-2 gap-3">
          <UFormField label="Date of birth" hint="Used to calculate your current age">
            <DateInput
              :model-value="form.dateOfBirth ?? ''"
              @update:model-value="form.dateOfBirth = $event || null"
              class="w-full"
            />
          </UFormField>
          <UFormField label="Target retirement age">
            <UInput v-model.number="form.targetRetirementAge" type="number" class="w-full" />
          </UFormField>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <UFormField label="Annual expenses target">
            <CurrencyInput v-model="form.annualExpensesTarget" class="w-full" />
          </UFormField>
          <UFormField label="Annual income">
            <CurrencyInput v-model="form.annualIncome" class="w-full" />
          </UFormField>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <UFormField label="Lean FIRE expenses (optional)">
            <CurrencyInput v-model="form.leanFireAnnualExpenses" class="w-full" />
          </UFormField>
          <UFormField label="Fat FIRE expenses (optional)">
            <CurrencyInput v-model="form.fatFireAnnualExpenses" class="w-full" />
          </UFormField>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <UFormField label="Expected return rate">
            <PercentInput v-model="form.expectedReturnRate" class="w-full" />
          </UFormField>
          <UFormField label="Inflation rate">
            <PercentInput v-model="form.inflationRate" class="w-full" />
          </UFormField>
        </div>
        <UFormField label="HSA coverage">
          <USelect
            v-model="form.hsaCoverage"
            :items="[
              { label: 'Self-only', value: 'self' },
              { label: 'Family', value: 'family' },
            ]"
            class="w-44"
          />
        </UFormField>
        <UButton type="submit" :loading="savingProfile" :disabled="savingProfile">Save</UButton>
      </UForm>
    </section>

    <hr class="border-default" />

    <section class="space-y-3">
      <h2 class="text-xl font-bold">Cloud Sync (Turso)</h2>
      <p class="text-sm text-muted">
        Optional. Keeps a synced cloud backup in your own free Turso database and
        reconciles your data across machines. Data is sent over an encrypted connection,
        the auth token is stored in your OS keychain, and your local database file is
        protected by your operating system's full-disk encryption. The app works fully
        offline without this.
      </p>

      <div class="text-sm">
        Status:
        <span class="font-medium">{{ isSynced ? 'Syncing' : 'Local only' }}</span>
        <span v-if="isSynced"> · last synced {{ lastSynced }}</span>
        <span v-if="syncStore.status?.status === 'syncing'"> · syncing…</span>
        <span v-if="syncStore.status?.lastError" class="text-error">
          · {{ syncStore.status.lastError }}
        </span>
      </div>

      <template v-if="!isSynced">
        <UFormField label="Database URL">
          <UInput v-model="syncUrl" placeholder="libsql://your-db-name-you.turso.io" class="w-full" />
        </UFormField>
        <UFormField label="Auth token (treated like a password)">
          <UInput v-model="syncToken" type="password" class="w-full" />
        </UFormField>
        <UButton :loading="syncBusy" @click="enableSync">Enable sync</UButton>
      </template>

      <template v-else>
        <div class="flex gap-2">
          <UButton :loading="syncBusy" @click="runSyncNow">Sync now</UButton>
          <UButton color="error" variant="soft" :loading="syncBusy" @click="disableSync">
            Disable sync
          </UButton>
        </div>
      </template>

      <p v-if="syncMessage" class="text-sm text-error" aria-live="polite">{{ syncMessage }}</p>

      <UAccordion
        :items="[{ label: 'How to set this up', slot: 'help', icon: 'i-ph-question' }]"
      >
        <template #help>
          <div class="text-sm space-y-4 p-2">
            <div>
              <h3 class="font-semibold mb-1">Option A — Turso website (no terminal)</h3>
              <ol class="list-decimal ml-5 space-y-1">
                <li>Go to <span class="font-mono">turso.tech</span> and create a free account.</li>
                <li>Click <strong>Create Database</strong> and give it a name (e.g. <em>trackmyfi</em>).</li>
                <li>Open the database, find <strong>Database URL</strong>, and copy it (starts with <span class="font-mono">libsql://</span>).</li>
                <li>Create a database token (look for <strong>Tokens</strong> / <strong>Create Token</strong>) and copy it.</li>
                <li>Paste both above and click <strong>Enable sync</strong>.</li>
              </ol>
            </div>
            <div>
              <h3 class="font-semibold mb-1">Option B — Turso CLI (technical)</h3>
              <pre class="bg-elevated rounded p-2 overflow-x-auto"><code>turso auth signup
turso db create trackmyfi
turso db show trackmyfi --url        # the Database URL
turso db tokens create trackmyfi     # the auth token</code></pre>
              <p class="mt-1">Paste the URL and token above and click <strong>Enable sync</strong>.</p>
            </div>
            <p class="text-muted">
              It's free-tier. The URL isn't secret, but the token is — treat it like a password.
              You own this cloud database.
            </p>
          </div>
        </template>
      </UAccordion>
    </section>

    <hr class="border-default" />

    <section class="space-y-3">
      <h2 class="text-xl font-bold">Attachment Storage</h2>
      <p class="text-sm text-muted">
        Where receipts and files attached to asset events are stored. Local storage is private to
        this machine. Cloud providers let attachments follow you across devices when the same
        bucket is configured everywhere.
      </p>

      <div class="text-sm">
        Current:
        <span class="font-medium">{{ storageStore.providerLabel }}</span>
        <template v-if="storageStore.config?.bucketName">
          · <span class="font-mono">{{ storageStore.config.bucketName }}</span>
        </template>
        <template v-if="storageStore.provider === 'local' && storageStore.config?.localPath">
          <br />
          <span class="text-muted font-mono text-xs">{{ storageStore.config.localPath }}</span>
        </template>
      </div>

      <!-- Cross-device credential prompt: config synced from another device but no local keychain entry -->
      <div
        v-if="storageStore.needsCredentials"
        class="flex items-start gap-3 rounded-lg border border-warning bg-warning/10 px-4 py-3 text-sm"
      >
        <span class="i-ph-key-duotone mt-0.5 shrink-0 text-warning text-base" />
        <div class="space-y-1">
          <p class="font-medium text-warning">Credentials needed on this device</p>
          <p class="text-muted">
            {{ storageStore.providerLabel }} is configured (synced from another device) but no
            credentials have been entered here yet. Enter them below and click
            <strong>Save storage settings</strong> to enable attachments on this device.
          </p>
        </div>
      </div>

      <UFormField label="Provider">
        <USelect v-model="storageProvider" :items="PROVIDER_OPTIONS" class="w-72" />
      </UFormField>

      <!-- R2 fields -->
      <template v-if="storageProvider === 'r2'">
        <UFormField label="Account ID">
          <UInput v-model="storageR2AccountId" placeholder="abc123..." class="w-full" />
        </UFormField>
        <UFormField label="Bucket name">
          <UInput v-model="storageBucket" placeholder="trackmyfi-attachments" class="w-full" />
        </UFormField>
        <UFormField label="Access Key ID">
          <UInput v-model="storageAccessKey" placeholder="Leave blank to keep existing" class="w-full" />
        </UFormField>
        <UFormField label="Secret Access Key">
          <UInput v-model="storageSecretKey" type="password" placeholder="Leave blank to keep existing" class="w-full" />
        </UFormField>
      </template>

      <!-- GCS fields -->
      <template v-else-if="storageProvider === 'gcs'">
        <UFormField label="Bucket name">
          <UInput v-model="storageBucket" placeholder="trackmyfi-attachments" class="w-full" />
        </UFormField>
        <UFormField label="Service account JSON key" hint="Paste the full JSON from your GCP service account key file">
          <UTextarea v-model="storageServiceAccountJson" :rows="6" placeholder='{"type":"service_account",...}' class="w-full font-mono text-xs" />
        </UFormField>
      </template>

      <!-- S3 fields -->
      <template v-else-if="storageProvider === 's3'">
        <UFormField label="Bucket name">
          <UInput v-model="storageBucket" placeholder="trackmyfi-attachments" class="w-full" />
        </UFormField>
        <UFormField label="Region">
          <UInput v-model="storageS3Region" placeholder="us-east-1" class="w-full" />
        </UFormField>
        <UFormField label="Access Key ID">
          <UInput v-model="storageAccessKey" placeholder="Leave blank to keep existing" class="w-full" />
        </UFormField>
        <UFormField label="Secret Access Key">
          <UInput v-model="storageSecretKey" type="password" placeholder="Leave blank to keep existing" class="w-full" />
        </UFormField>
      </template>

      <div class="flex gap-2">
        <UButton :loading="storageBusy" @click="saveStorageSettings">Save storage settings</UButton>
        <UButton
          v-if="storageStore.provider !== 'local'"
          color="error"
          variant="soft"
          :loading="storageBusy"
          @click="clearStorageSettings"
        >
          Reset to local
        </UButton>
      </div>

      <p v-if="storageMessage" class="text-sm text-error" aria-live="polite">{{ storageMessage }}</p>
    </section>

    <hr class="border-default" />

    <section class="space-y-3">
      <h2 class="text-xl font-bold">Category Rules</h2>
      <p class="text-sm text-muted">
        Keywords matched against transaction descriptions during CSV import.
        First matching rule wins; unmatched rows use the mapping's default category.
      </p>

      <UTable :data="categoryRules" :columns="ruleColumns" empty="No rules yet.">
        <template #keyword-cell="{ row }">
          <span class="font-mono text-xs">{{ row.original.keyword }}</span>
        </template>
        <template #actions-cell="{ row }">
          <UButton size="xs" color="error" variant="ghost" :loading="removingRuleId === row.original.id" :disabled="removingRuleId !== null" @click="removeCategoryRule(row.original.id)">
            Remove
          </UButton>
        </template>
      </UTable>

      <div class="flex gap-2 items-center pt-1">
        <UInput
          v-model="newRuleKeyword"
          placeholder="keyword (e.g. netflix)"
          class="flex-1"
          @keydown.enter="addCategoryRule"
        />
        <USelect v-model="newRuleCategory" :items="categoryItems" class="w-44" />
        <UButton size="sm" variant="soft" :loading="savingRule" :disabled="!newRuleKeyword.trim() || savingRule" @click="addCategoryRule">
          Add rule
        </UButton>
      </div>
    </section>

    <hr class="border-default" />

    <section class="space-y-3">
      <h2 class="text-xl font-bold">Vendor Rules</h2>
      <p class="text-sm text-muted">
        Keywords matched against transaction descriptions on the Expenses page.
        When a description contains the keyword, its vendor name is used instead
        of the auto-detected one — handy for messy or inconsistent bank descriptions.
        If more than one rule matches, the longest keyword wins.
      </p>

      <UTable :data="vendorRules" :columns="vendorRuleColumns" empty="No rules yet.">
        <template #keyword-cell="{ row }">
          <span class="font-mono text-xs">{{ row.original.keyword }}</span>
        </template>
        <template #actions-cell="{ row }">
          <UButton size="xs" color="error" variant="ghost" :loading="removingVendorRuleId === row.original.id" :disabled="removingVendorRuleId !== null" @click="removeVendorRule(row.original.id)">
            Remove
          </UButton>
        </template>
      </UTable>

      <div class="flex gap-2 items-center pt-1">
        <UInput
          v-model="newVendorKeyword"
          placeholder="keyword (e.g. pizza hut)"
          class="flex-1"
          @keydown.enter="addVendorRule"
        />
        <UInput
          v-model="newVendorName"
          placeholder="vendor name (e.g. Pizza Hut)"
          class="flex-1"
          @keydown.enter="addVendorRule"
        />
        <UButton size="sm" variant="soft" :loading="savingVendorRule" :disabled="!newVendorKeyword.trim() || !newVendorName.trim() || savingVendorRule" @click="addVendorRule">
          Add rule
        </UButton>
      </div>

      <div v-if="vendorRuleSuggestions.length" class="space-y-2 pt-3 mt-2 border-t border-default">
        <p class="text-xs font-medium text-muted uppercase tracking-wide">Suggested from your transactions</p>
        <div
          v-for="s in vendorRuleSuggestions"
          :key="s.key"
          class="flex items-center justify-between gap-3 rounded-lg border border-dashed border-default px-3 py-2"
        >
          <div class="min-w-0">
            <p class="text-sm text-heading">
              <span class="font-medium">{{ s.vendorName }}</span>
              <span class="text-xs text-dimmed"> — keyword "{{ s.keyword }}"</span>
            </p>
            <p class="text-xs text-muted truncate">
              {{ s.count }} transactions · {{ money(s.total) }} · e.g. {{ s.sampleDescriptions.join(', ') }}
            </p>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <UButton size="xs" variant="ghost" color="neutral" @click="dismissVendorRuleSuggestion(s)">Dismiss</UButton>
            <UButton
              size="xs"
              variant="soft"
              :loading="addingSuggestionKey === s.key"
              :disabled="addingSuggestionKey !== null"
              @click="acceptVendorRuleSuggestion(s)"
            >
              Add rule
            </UButton>
          </div>
        </div>
      </div>
    </section>

    <hr class="border-default" />

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

    <DeleteDataModal v-model:open="showDeleteModal" />
  </div>
</template>
