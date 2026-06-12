<script setup lang="ts">
import { reactive, watch } from 'vue'
import { DateTime } from 'luxon'
import { useAccountsStore } from '../stores/accounts'
import { ACCOUNT_TYPES, defaultIncludeInFire, type AccountType } from '../lib/accountTypes'

const store = useAccountsStore()

const form = reactive({
  name: '',
  type: 'checking' as AccountType,
  institution: '',
  includeInFireCalculations: false,
})

watch(
  () => form.type,
  (newType) => {
    form.includeInFireCalculations = defaultIncludeInFire(newType)
  },
  { immediate: true },
)

const accountTypeItems = ACCOUNT_TYPES.map((t) => ({ label: t, value: t }))

async function onSubmit() {
  await store.create({
    name: form.name,
    type: form.type,
    institution: form.institution.trim() || null,
    includeInFireCalculations: form.includeInFireCalculations,
    createdAt: DateTime.now().toISODate()!,
  })
  form.name = ''
  form.type = 'checking'
  form.institution = ''
  form.includeInFireCalculations = false
}
</script>

<template>
  <UCard class="mb-6">
    <template #header>
      <h2 class="text-lg font-semibold">Add Account</h2>
    </template>
    <UForm :state="form" @submit="onSubmit" class="space-y-3">
      <UFormField label="Name" required>
        <UInput v-model="form.name" placeholder="e.g. Fidelity Brokerage" />
      </UFormField>
      <UFormField label="Type" required>
        <USelect
          v-model="form.type"
          :items="accountTypeItems"
          value-key="value"
          placeholder="Select account type"
        />
      </UFormField>
      <UFormField label="Institution (optional)">
        <UInput v-model="form.institution" placeholder="e.g. Fidelity" />
      </UFormField>
      <UFormField label="Include in FIRE calculations">
        <USwitch v-model="form.includeInFireCalculations" />
      </UFormField>
      <UButton type="submit" :disabled="!form.name">Add Account</UButton>
    </UForm>
  </UCard>
</template>
