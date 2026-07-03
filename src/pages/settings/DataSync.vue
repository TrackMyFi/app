<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useSyncStore } from '../../stores/sync'
import { useStorageStore } from '../../stores/storage'
import {
  saveSyncConfig,
  clearSyncConfig,
  syncNow,
  restartApp,
} from '../../lib/api/sync'
import { countMigratableAttachments, migrateAndSaveStorageConfig } from '../../lib/api/storage'
import PageError from '../../components/PageError.vue'
import SettingsNav from '../../components/SettingsNav.vue'
import { usePageData } from '../../composables/usePageData'
import { useToast } from '@nuxt/ui/composables'

const toast = useToast()
const { error, run, retry } = usePageData()

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

onMounted(() => run(async () => {
  await storageStore.load()
  if (storageStore.config) {
    storageProvider.value = storageStore.config.provider || 'local'
    storageBucket.value = storageStore.config.bucketName ?? ''
    storageR2AccountId.value = storageStore.config.r2AccountId ?? ''
    storageS3Region.value = storageStore.config.s3Region ?? ''
  }
}))

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
            title: `Migrated ${result.migrated} ${Number(result.migrated) === 1 ? 'attachment' : 'attachments'} to ${providerLabel(newProvider)}${failNote}`,
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
  <div class="p-6 max-w-7xl">
    <h1 class="text-2xl font-bold mb-4">Settings</h1>
    <SettingsNav />

    <PageError v-if="error" :message="error" @retry="retry" />

    <div v-else class="space-y-8 max-w-3xl">
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
    </div>
  </div>
</template>
