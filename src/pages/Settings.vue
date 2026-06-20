<script setup lang="ts">
import { reactive, ref, computed, onMounted } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { confirm } from '@tauri-apps/plugin-dialog'
import { DateTime } from 'luxon'
import { useFireProfileStore } from '../stores/fireProfile'
import { useSyncStore } from '../stores/sync'
import { useUpdaterStore } from '../stores/updater'
import {
  saveSyncConfig,
  clearSyncConfig,
  syncNow,
  restartApp,
} from '../lib/api/sync'
import * as categoryRulesApi from '../lib/api/categoryRules'
import type { FireProfile } from '../lib/types/FireProfile'
import type { CategoryRule } from '../lib/types/CategoryRule'
import DeleteDataModal from '../components/DeleteDataModal.vue'
import CurrencyInput from '../components/CurrencyInput.vue'
import PercentInput from '../components/PercentInput.vue'
import DateInput from '../components/DateInput.vue'
import { categoryItems, labelForCategory } from '../lib/transactions/constants'

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

async function checkForUpdates() {
  upToDate.value = false
  await updater.check()
  // The popover handles the "available" case; surface the all-clear here.
  if (updater.status === 'idle') upToDate.value = true
}

onMounted(async () => {
  await store.load()
  if (store.profile) Object.assign(form, store.profile)
  categoryRules.value = await categoryRulesApi.listCategoryRules()
  await updater.loadVersion()
})

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

const syncStore = useSyncStore()
const syncUrl = ref('')
const syncToken = ref('')
const syncBusy = ref(false)
const syncMessage = ref('')

const showDeleteModal = ref(false)

const categoryRules = ref<CategoryRule[]>([])
const newRuleKeyword = ref('')
const newRuleCategory = ref('discretionary')
const savingRule = ref(false)
const removingRuleId = ref<number | null>(null)

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
</script>

<template>
  <div class="p-6 max-w-3xl space-y-8">
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
