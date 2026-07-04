<script setup lang="ts">
import { reactive, ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useFireProfileStore } from '../stores/fireProfile'
import { useStorageStore } from '../stores/storage'
import { markOnboardingComplete, upsertFireProfile } from '../lib/api/fireProfile'
import { saveSyncConfig, restartApp } from '../lib/api/sync'
import type { FireProfile } from '../lib/types/FireProfile'
import CurrencyInput from '../components/CurrencyInput.vue'
import PercentInput from '../components/PercentInput.vue'
import DateInput from '../components/DateInput.vue'

const router = useRouter()
const fireProfileStore = useFireProfileStore()
const storageStore = useStorageStore()

const step = ref<1 | 2 | 3 | 4 | 5 | 6>(1)
const TOTAL_STEPS = 6

const form = reactive({
  dateOfBirth: null as string | null,
  targetRetirementAge: 50,
  annualExpensesTarget: 40000,
  leanFireAnnualExpenses: null as number | null,
  fatFireAnnualExpenses: null as number | null,
  annualIncome: 80000,
  expectedReturnRate: 0.07,
  inflationRate: 0.03,
  hsaCoverage: 'self',
})

const syncEnabled = ref(false)
const syncUrl = ref('')
const syncToken = ref('')
const syncBusy = ref(false)
const syncError = ref('')
const showSyncHelp = ref(false)

// Step 6: Attachment Storage
const storageEnabled = ref(false)
const storageProvider = ref('r2')
const storageBucket = ref('')
const storageR2AccountId = ref('')
const storageS3Region = ref('')
const storageAccessKey = ref('')
const storageSecretKey = ref('')
const storageServiceAccountJson = ref('')

const STORAGE_PROVIDER_OPTIONS = [
  { label: 'Cloudflare R2 (free egress, recommended)', value: 'r2' },
  { label: 'Google Cloud Storage', value: 'gcs' },
  { label: 'Amazon S3', value: 's3' },
]

onMounted(async () => {
  if (fireProfileStore.profile) {
    const p = fireProfileStore.profile
    form.dateOfBirth = p.dateOfBirth
    form.targetRetirementAge = p.targetRetirementAge
    form.annualExpensesTarget = p.annualExpensesTarget
    form.leanFireAnnualExpenses = p.leanFireAnnualExpenses
    form.fatFireAnnualExpenses = p.fatFireAnnualExpenses
    form.annualIncome = p.annualIncome
    form.expectedReturnRate = p.expectedReturnRate
    form.inflationRate = p.inflationRate
    form.hsaCoverage = p.hsaCoverage
  }
})

const progressWidth = computed(() => `${((step.value - 1) / (TOTAL_STEPS - 1)) * 100}%`)

function next() {
  if (step.value < TOTAL_STEPS) step.value = (step.value + 1) as 1 | 2 | 3 | 4 | 5 | 6
}

function back() {
  if (step.value > 1) step.value = (step.value - 1) as 1 | 2 | 3 | 4 | 5 | 6
}

async function skip() {
  await markOnboardingComplete()
  await fireProfileStore.load()
  router.push('/')
}

async function finish() {
  syncBusy.value = true
  syncError.value = ''
  try {
    const profile: FireProfile = {
      dateOfBirth: form.dateOfBirth || null,
      targetRetirementAge: form.targetRetirementAge,
      annualExpensesTarget: form.annualExpensesTarget ?? 0,
      leanFireAnnualExpenses: form.leanFireAnnualExpenses,
      fatFireAnnualExpenses: form.fatFireAnnualExpenses,
      annualIncome: form.annualIncome ?? 0,
      expectedReturnRate: form.expectedReturnRate ?? 0,
      inflationRate: form.inflationRate ?? 0,
      // Not asked during onboarding — the 4% rule default; tunable in Settings.
      withdrawalRate: fireProfileStore.profile?.withdrawalRate ?? 0.04,
      hsaCoverage: form.hsaCoverage,
      onboardingCompleted: false,
    }
    await upsertFireProfile(profile)

    if (storageEnabled.value) {
      await storageStore.save({
        provider: storageProvider.value,
        bucketName: storageBucket.value.trim() || null,
        r2AccountId: storageR2AccountId.value.trim() || null,
        s3Region: storageS3Region.value.trim() || null,
        accessKeyId: storageAccessKey.value.trim() || null,
        secretAccessKey: storageSecretKey.value.trim() || null,
        serviceAccountJson: storageServiceAccountJson.value.trim() || null,
      })
    }

    if (syncEnabled.value) {
      if (!syncUrl.value || !syncToken.value) {
        syncError.value = 'Enter both the database URL and the auth token to enable sync.'
        return
      }
      // Mark complete before saveSyncConfig so it's copied to Turso during seeding.
      // saveSyncConfig renames the local DB file, making the app's Db state stale —
      // restart instead of trying to use it afterward.
      await markOnboardingComplete()
      await saveSyncConfig(syncUrl.value.trim(), syncToken.value.trim())
      await restartApp()
      return
    }

    await markOnboardingComplete()
    await fireProfileStore.load()
    router.push('/')
  } catch (e) {
    syncError.value = String(e)
  } finally {
    syncBusy.value = false
  }
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-background p-8">
    <div class="w-full max-w-lg">
      <div class="flex items-center justify-between mb-2">
        <span class="text-sm text-muted">Step {{ step }} of {{ TOTAL_STEPS }}</span>
        <UButton variant="ghost" size="sm" @click="skip">Skip setup</UButton>
      </div>

      <div class="w-full bg-elevated rounded-full h-1.5 mb-8">
        <div
          class="bg-primary h-1.5 rounded-full transition-all duration-300"
          :style="{ width: progressWidth }"
        />
      </div>

      <!-- Step 1: Ages -->
      <div v-if="step === 1" class="space-y-6">
        <div>
          <img src="/logo-icon.svg" alt="TrackMyFI" class="w-12 h-12 mb-4" />
          <h1 class="text-2xl font-bold">Welcome to TrackMyFI</h1>
          <p class="text-muted mt-1">Let's set up your FIRE profile. You can change any of this later in Settings.</p>
        </div>
        <UFormField label="Date of birth" hint="Used to calculate and auto-update your current age">
          <DateInput
            :model-value="form.dateOfBirth ?? ''"
            @update:model-value="form.dateOfBirth = $event || null"
            class="w-full"
          />
        </UFormField>
        <UFormField label="When would you like to retire? (age)">
          <UInput v-model.number="form.targetRetirementAge" type="number" class="w-full" />
        </UFormField>
      </div>

      <!-- Step 2: Expenses -->
      <div v-else-if="step === 2" class="space-y-6">
        <div>
          <h1 class="text-2xl font-bold">Your expenses</h1>
          <p class="text-muted mt-1">How much do you spend per year in retirement?</p>
        </div>
        <UFormField label="Annual living expenses target">
          <CurrencyInput v-model="form.annualExpensesTarget" class="w-full" />
        </UFormField>
        <div class="border border-default rounded-lg p-4 space-y-4">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium">Lean & Fat FIRE targets</span>
            <span class="text-xs text-muted bg-elevated px-2 py-0.5 rounded-full">Optional</span>
          </div>
          <p class="text-xs text-muted">Define a minimum (lean) and comfortable (fat) spending target for more nuanced projections. You can set these anytime in Settings.</p>
          <UFormField label="Lean FIRE annual expenses">
            <CurrencyInput v-model="form.leanFireAnnualExpenses" class="w-full" />
          </UFormField>
          <UFormField label="Fat FIRE annual expenses">
            <CurrencyInput v-model="form.fatFireAnnualExpenses" class="w-full" />
          </UFormField>
        </div>
      </div>

      <!-- Step 3: Income & Growth -->
      <div v-else-if="step === 3" class="space-y-6">
        <div>
          <h1 class="text-2xl font-bold">Income & growth</h1>
          <p class="text-muted mt-1">These numbers power your FIRE projections.</p>
        </div>
        <UFormField label="Annual income">
          <CurrencyInput v-model="form.annualIncome" class="w-full" />
        </UFormField>
        <UFormField label="Expected annual return rate" hint="The average yearly return on your investments. A common estimate is 7% for a diversified portfolio.">
          <PercentInput v-model="form.expectedReturnRate" class="w-full" />
        </UFormField>
        <UFormField label="Expected inflation rate" hint="How much purchasing power erodes each year. 3% is a common estimate.">
          <PercentInput v-model="form.inflationRate" class="w-full" />
        </UFormField>
      </div>

      <!-- Step 4: HSA -->
      <div v-else-if="step === 4" class="space-y-6">
        <div>
          <h1 class="text-2xl font-bold">Healthcare coverage</h1>
          <p class="text-muted mt-1">Do you have a Health Savings Account (HSA)?</p>
        </div>
        <UFormField label="HSA coverage">
          <USelect
            v-model="form.hsaCoverage"
            :items="[
              { label: 'Self-only', value: 'self' },
              { label: 'Family', value: 'family' },
            ]"
            class="w-56"
          />
        </UFormField>
      </div>

      <!-- Step 5: Cloud Sync -->
      <div v-else-if="step === 5" class="space-y-6">
        <div>
          <h1 class="text-2xl font-bold">Cloud sync</h1>
          <p class="text-muted mt-1">
            Optionally back up and sync your data to your own Turso database.
            The app works fully offline without this.
          </p>
        </div>

        <div class="flex items-center gap-3">
          <USwitch v-model="syncEnabled" />
          <span class="text-sm font-medium">Enable cloud sync</span>
          <UButton
            icon="i-ph-question"
            variant="ghost"
            size="xs"
            @click="showSyncHelp = true"
          />
        </div>

        <template v-if="syncEnabled">
          <p class="text-xs text-muted">Requires your own free <strong>Turso</strong> account.</p>
          <UFormField label="Database URL">
            <UInput v-model="syncUrl" placeholder="libsql://your-db-name-you.turso.io" class="w-full" />
          </UFormField>
          <UFormField label="Auth token (treated like a password)">
            <UInput v-model="syncToken" type="password" class="w-full" />
          </UFormField>
          <p v-if="syncError" class="text-sm text-error" aria-live="polite">{{ syncError }}</p>
        </template>
      </div>

      <!-- Step 6: Attachment Storage -->
      <div v-else-if="step === 6" class="space-y-6">
        <div>
          <h1 class="text-2xl font-bold">Attachment storage</h1>
          <p class="text-muted mt-1">
            Attach receipts and files to asset events. By default files are stored locally.
            Enable cloud storage to access attachments from any device.
          </p>
        </div>

        <div class="flex items-center gap-3">
          <USwitch v-model="storageEnabled" />
          <span class="text-sm font-medium">Store attachments in the cloud</span>
        </div>

        <template v-if="storageEnabled">
          <UFormField label="Provider">
            <USelect v-model="storageProvider" :items="STORAGE_PROVIDER_OPTIONS" class="w-full" />
          </UFormField>

          <template v-if="storageProvider === 'r2'">
            <UFormField label="Account ID">
              <UInput v-model="storageR2AccountId" placeholder="abc123…" class="w-full" />
            </UFormField>
            <UFormField label="Bucket name">
              <UInput v-model="storageBucket" placeholder="trackmyfi-attachments" class="w-full" />
            </UFormField>
            <UFormField label="Access Key ID">
              <UInput v-model="storageAccessKey" class="w-full" />
            </UFormField>
            <UFormField label="Secret Access Key">
              <UInput v-model="storageSecretKey" type="password" class="w-full" />
            </UFormField>
          </template>

          <template v-else-if="storageProvider === 'gcs'">
            <UFormField label="Bucket name">
              <UInput v-model="storageBucket" placeholder="trackmyfi-attachments" class="w-full" />
            </UFormField>
            <UFormField label="Service account JSON key">
              <UTextarea v-model="storageServiceAccountJson" :rows="5" placeholder='{"type":"service_account",...}' class="w-full font-mono text-xs" />
            </UFormField>
          </template>

          <template v-else-if="storageProvider === 's3'">
            <UFormField label="Bucket name">
              <UInput v-model="storageBucket" placeholder="trackmyfi-attachments" class="w-full" />
            </UFormField>
            <UFormField label="Region">
              <UInput v-model="storageS3Region" placeholder="us-east-1" class="w-full" />
            </UFormField>
            <UFormField label="Access Key ID">
              <UInput v-model="storageAccessKey" class="w-full" />
            </UFormField>
            <UFormField label="Secret Access Key">
              <UInput v-model="storageSecretKey" type="password" class="w-full" />
            </UFormField>
          </template>

          <p class="text-xs text-muted">
            Credentials are stored in your OS keychain and never leave this device.
            You can change these anytime in Settings.
          </p>
        </template>

        <p v-if="!storageEnabled" class="text-xs text-muted border border-default rounded-lg p-3">
          Files will be saved to <span class="font-mono">~/Library/Application Support/com.trackmyfi.desktop/attachments/</span>.
          They stay on this machine only.
        </p>
      </div>

      <!-- Navigation -->
      <div class="flex items-center justify-between mt-8">
        <UButton v-if="step > 1" variant="ghost" @click="back">Back</UButton>
        <div v-else />
        <UButton v-if="step < TOTAL_STEPS" @click="next">Next</UButton>
        <UButton v-else :loading="syncBusy" @click="finish">Finish</UButton>
      </div>
    </div>

    <!-- Sync Help Modal -->
    <UModal v-model:open="showSyncHelp" title="How to set up Turso sync">
      <template #body>
        <div class="text-sm space-y-4">
          <div>
            <h3 class="font-semibold mb-1">Option A — Turso website (no terminal)</h3>
            <ol class="list-decimal ml-5 space-y-1">
              <li>Go to <span class="font-mono">turso.tech</span> and create a free account.</li>
              <li>Click <strong>Create Database</strong> and give it a name (e.g. <em>trackmyfi</em>).</li>
              <li>Open the database, find <strong>Database URL</strong>, and copy it (starts with <span class="font-mono">libsql://</span>).</li>
              <li>Create a database token (look for <strong>Tokens</strong> / <strong>Create Token</strong>) and copy it.</li>
              <li>Paste both above and click <strong>Finish</strong>.</li>
            </ol>
          </div>
          <div>
            <h3 class="font-semibold mb-1">Option B — Turso CLI (technical)</h3>
            <pre class="bg-elevated rounded p-2 overflow-x-auto"><code>turso auth signup
turso db create trackmyfi
turso db show trackmyfi --url        # the Database URL
turso db tokens create trackmyfi     # the auth token</code></pre>
            <p class="mt-1">Paste the URL and token above and click <strong>Finish</strong>.</p>
          </div>
          <p class="text-muted">
            It's free-tier. The URL isn't secret, but the token is — treat it like a password.
            You own this cloud database.
          </p>
        </div>
      </template>
    </UModal>
  </div>
</template>
