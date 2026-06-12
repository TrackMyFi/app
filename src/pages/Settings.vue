<script setup lang="ts">
import { reactive, onMounted } from 'vue'
import { useFireProfileStore } from '../stores/fireProfile'
import type { FireProfile } from '../lib/types/FireProfile'

interface FireProfileForm {
  currentAge: number
  targetRetirementAge: number
  annualExpensesTarget: number
  leanFireAnnualExpenses: number | null
  fatFireAnnualExpenses: number | null
  annualIncome: number
  expectedReturnRate: number
  inflationRate: number
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
})

onMounted(async () => {
  await store.load()
  if (store.profile) Object.assign(form, store.profile)
})

async function onSubmit() {
  const profile: FireProfile = {
    currentAge: form.currentAge,
    targetRetirementAge: form.targetRetirementAge,
    annualExpensesTarget: form.annualExpensesTarget,
    leanFireAnnualExpenses: form.leanFireAnnualExpenses,
    fatFireAnnualExpenses: form.fatFireAnnualExpenses,
    annualIncome: form.annualIncome,
    expectedReturnRate: form.expectedReturnRate,
    inflationRate: form.inflationRate,
  }
  await store.save(profile)
}
</script>

<template>
  <div class="p-6 max-w-xl">
    <h1 class="text-2xl font-bold mb-4">FIRE Profile</h1>
    <UForm :state="form" @submit="onSubmit" class="space-y-3">
      <UFormField label="Current age">
        <UInput v-model.number="form.currentAge" type="number" />
      </UFormField>
      <UFormField label="Target retirement age">
        <UInput v-model.number="form.targetRetirementAge" type="number" />
      </UFormField>
      <UFormField label="Annual expenses target">
        <UInput v-model.number="form.annualExpensesTarget" type="number" />
      </UFormField>
      <UFormField label="Lean FIRE expenses (optional)">
        <UInput v-model.number="form.leanFireAnnualExpenses" type="number" />
      </UFormField>
      <UFormField label="Fat FIRE expenses (optional)">
        <UInput v-model.number="form.fatFireAnnualExpenses" type="number" />
      </UFormField>
      <UFormField label="Annual income">
        <UInput v-model.number="form.annualIncome" type="number" />
      </UFormField>
      <UFormField label="Expected return rate (e.g. 0.07)">
        <UInput v-model.number="form.expectedReturnRate" type="number" step="0.01" />
      </UFormField>
      <UFormField label="Inflation rate (e.g. 0.03)">
        <UInput v-model.number="form.inflationRate" type="number" step="0.01" />
      </UFormField>
      <UButton type="submit">Save</UButton>
    </UForm>
  </div>
</template>
