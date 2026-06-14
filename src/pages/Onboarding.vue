<script setup lang="ts">
import { reactive, ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useFireProfileStore } from '../stores/fireProfile'
import { useSyncStore } from '../stores/sync'
import { markOnboardingComplete, upsertFireProfile } from '../lib/api/fireProfile'
import { saveSyncConfig } from '../lib/api/sync'
import type { FireProfile } from '../lib/types/FireProfile'

const router = useRouter()
const fireProfileStore = useFireProfileStore()
const syncStore = useSyncStore()

const step = ref<1 | 2 | 3 | 4 | 5>(1)
const TOTAL_STEPS = 5

const form = reactive({
  currentAge: 30,
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

onMounted(async () => {
  if (fireProfileStore.profile) {
    const p = fireProfileStore.profile
    form.currentAge = p.currentAge
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
  if (step.value < TOTAL_STEPS) step.value = (step.value + 1) as typeof step.value
}

function back() {
  if (step.value > 1) step.value = (step.value - 1) as typeof step.value
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
      currentAge: form.currentAge,
      targetRetirementAge: form.targetRetirementAge,
      annualExpensesTarget: form.annualExpensesTarget,
      leanFireAnnualExpenses: form.leanFireAnnualExpenses,
      fatFireAnnualExpenses: form.fatFireAnnualExpenses,
      annualIncome: form.annualIncome,
      expectedReturnRate: form.expectedReturnRate,
      inflationRate: form.inflationRate,
      hsaCoverage: form.hsaCoverage,
      onboardingCompleted: false,
    }
    await upsertFireProfile(profile)

    if (syncEnabled.value) {
      if (!syncUrl.value || !syncToken.value) {
        syncError.value = 'Enter both the database URL and the auth token to enable sync.'
        return
      }
      await saveSyncConfig(syncUrl.value.trim(), syncToken.value.trim())
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
          <h1 class="text-2xl font-bold">Welcome to TrackMyFI</h1>
          <p class="text-muted mt-1">Let's set up your FIRE profile. You can change any of this later in Settings.</p>
        </div>
        <UFormField label="How old are you?">
          <UInput v-model.number="form.currentAge" type="number" class="w-full" />
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
          <UInput v-model.number="form.annualExpensesTarget" type="number" class="w-full" />
        </UFormField>
        <div class="border border-default rounded-lg p-4 space-y-4">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium">Lean & Fat FIRE targets</span>
            <span class="text-xs text-muted bg-elevated px-2 py-0.5 rounded-full">Optional</span>
          </div>
          <p class="text-xs text-muted">Define a minimum (lean) and comfortable (fat) spending target for more nuanced projections. You can set these anytime in Settings.</p>
          <UFormField label="Lean FIRE annual expenses">
            <UInput v-model.number="form.leanFireAnnualExpenses" type="number" class="w-full" placeholder="e.g. 30000" />
          </UFormField>
          <UFormField label="Fat FIRE annual expenses">
            <UInput v-model.number="form.fatFireAnnualExpenses" type="number" class="w-full" placeholder="e.g. 80000" />
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
          <UInput v-model.number="form.annualIncome" type="number" class="w-full" />
        </UFormField>
        <UFormField label="Expected annual return rate" hint="The average yearly return on your investments. A common estimate is 7% (0.07) for a diversified portfolio.">
          <UInput v-model.number="form.expectedReturnRate" type="number" step="0.01" class="w-full" placeholder="0.07" />
        </UFormField>
        <UFormField label="Expected inflation rate" hint="How much purchasing power erodes each year. 3% (0.03) is a common estimate.">
          <UInput v-model.number="form.inflationRate" type="number" step="0.01" class="w-full" placeholder="0.03" />
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
            icon="i-lucide-circle-help"
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
