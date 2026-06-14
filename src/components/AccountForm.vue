<script setup lang="ts">
import { reactive, watch } from 'vue'
import { DateTime } from 'luxon'
import { useAccountsStore } from '../stores/accounts'
import { ACCOUNT_TYPES, defaultIncludeInFire, type AccountType } from '../lib/accountTypes'
import type { Account } from '../lib/types/Account'
import DateInput from './DateInput.vue'

const props = defineProps<{ account?: Account }>()
const emit = defineEmits<{ saved: [] }>()

const store = useAccountsStore()
const isEdit = !!props.account

const form = reactive({
  name: props.account?.name ?? '',
  type: (props.account?.type ?? 'checking') as AccountType,
  institution: props.account?.institution ?? '',
  includeInFireCalculations: props.account?.includeInFireCalculations ?? false,
  createdAt: props.account?.createdAt ?? DateTime.now().toISODate()!,
})

// Auto-default the FIRE toggle from the account type ONLY when adding, so editing
// an existing account never silently flips a user's stored choice.
if (!isEdit) {
  watch(
    () => form.type,
    (newType) => {
      form.includeInFireCalculations = defaultIncludeInFire(newType)
    },
    { immediate: true },
  )
}

const accountTypeItems = ACCOUNT_TYPES.map((t) => ({ label: t, value: t }))

async function onSubmit() {
  const payload = {
    name: form.name,
    type: form.type,
    institution: form.institution.trim() || null,
    includeInFireCalculations: form.includeInFireCalculations,
    createdAt: form.createdAt,
  }
  if (isEdit) {
    await store.update(props.account!.id, payload)
  } else {
    await store.create(payload)
  }
  emit('saved')
}
</script>

<template>
  <UForm :state="form" @submit="onSubmit" class="space-y-4">
    <div class="grid grid-cols-2 gap-3">
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
    </div>
    <div class="grid grid-cols-2 gap-3">
      <UFormField label="Institution (optional)">
        <UInput v-model="form.institution" placeholder="e.g. Fidelity" />
      </UFormField>
      <UFormField label="Opened">
        <DateInput v-model="form.createdAt" />
      </UFormField>
    </div>
    <div class="flex items-center justify-between rounded-lg border border-default px-4 py-3">
      <span class="text-sm font-medium">Include in FIRE calculations</span>
      <USwitch v-model="form.includeInFireCalculations" />
    </div>
    <UButton type="submit" :disabled="!form.name">
      {{ isEdit ? 'Save Changes' : 'Add Account' }}
    </UButton>
  </UForm>
</template>
