<script setup lang="ts">
import { reactive, onMounted, ref } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { useFireProfileStore } from '../../stores/fireProfile'
import type { FireProfile } from '../../lib/types/FireProfile'
import CurrencyInput from '../../components/CurrencyInput.vue'
import PercentInput from '../../components/PercentInput.vue'
import DateInput from '../../components/DateInput.vue'
import PageError from '../../components/PageError.vue'
import SettingsNav from '../../components/SettingsNav.vue'
import { usePageData } from '../../composables/usePageData'

interface FireProfileForm {
  dateOfBirth: string | null
  targetRetirementAge: number
  annualExpensesTarget: number
  leanFireAnnualExpenses: number | null
  fatFireAnnualExpenses: number | null
  annualIncome: number
  expectedReturnRate: number
  inflationRate: number
  withdrawalRate: number
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
  withdrawalRate: 0.04,
  hsaCoverage: 'self',
})

const savingProfile = ref(false)

onMounted(() => run(async () => {
  await store.load()
  if (store.profile) Object.assign(form, store.profile)
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
    withdrawalRate: form.withdrawalRate || 0.04,
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
</script>

<template>
  <div class="p-6 max-w-7xl">
    <h1 class="text-2xl font-bold mb-4">Settings</h1>
    <SettingsNav />

    <PageError v-if="error" :message="error" @retry="retry" />

    <section v-else class="space-y-4 max-w-3xl">
      <h2 class="text-xl font-bold">FIRE Profile</h2>
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
        <div class="grid grid-cols-2 gap-3">
          <UFormField
            label="Safe withdrawal rate"
            hint="4% is the classic rule; long early retirements often plan on 3.5% or 3.25%"
          >
            <PercentInput v-model="form.withdrawalRate" :step="0.0025" class="w-full" />
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
  </div>
</template>
