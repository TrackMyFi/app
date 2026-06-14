<script setup lang="ts">
import { reactive, ref, computed, onMounted } from 'vue'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useFireProfileStore } from '../stores/fireProfile'
import { useSyncStore } from '../stores/sync'
import {
  saveSyncConfig,
  clearSyncConfig,
  syncNow,
  restartApp,
} from '../lib/api/sync'
import type { FireProfile } from '../lib/types/FireProfile'
import DeleteDataModal from '../components/DeleteDataModal.vue'
import CurrencyInput from '../components/CurrencyInput.vue'
import PercentInput from '../components/PercentInput.vue'

interface FireProfileForm {
  currentAge: number
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
const form = reactive<FireProfileForm>({
  currentAge: 0,
  targetRetirementAge: 0,
  annualExpensesTarget: 0,
  leanFireAnnualExpenses: null,
  fatFireAnnualExpenses: null,
  annualIncome: 0,
  expectedReturnRate: 0,
  inflationRate: 0,
  hsaCoverage: 'self',
})

onMounted(async () => {
  await store.load()
  if (store.profile) Object.assign(form, store.profile)
})

async function onSubmit() {
  const profile: FireProfile = {
    currentAge: form.currentAge,
    targetRetirementAge: form.targetRetirementAge,
    annualExpensesTarget: form.annualExpensesTarget ?? 0,
    leanFireAnnualExpenses: form.leanFireAnnualExpenses,
    fatFireAnnualExpenses: form.fatFireAnnualExpenses,
    annualIncome: form.annualIncome ?? 0,
    expectedReturnRate: form.expectedReturnRate ?? 0,
    inflationRate: form.inflationRate ?? 0,
    hsaCoverage: form.hsaCoverage,
  }
  await store.save(profile)
}

const syncStore = useSyncStore()
const syncUrl = ref('')
const syncToken = ref('')
const syncBusy = ref(false)
const syncMessage = ref('')

const showDeleteModal = ref(false)

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
</script>

<template>
  <div class="p-6 max-w-2xl space-y-8">
    <section class="space-y-4">
      <h1 class="text-2xl font-bold">FIRE Profile</h1>
      <UForm :state="form" @submit="onSubmit" class="space-y-4">
        <div class="grid grid-cols-2 gap-3">
          <UFormField label="Current age">
            <UInput v-model.number="form.currentAge" type="number" class="w-full" />
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
        <UButton type="submit">Save</UButton>
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
