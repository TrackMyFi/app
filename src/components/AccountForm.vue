<script setup lang="ts">
import { reactive, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { useAccountsStore } from '../stores/accounts'
import { accountTypeItems, defaultIncludeInFire, type AccountType } from '../lib/accountTypes'
import type { Account } from '../lib/types/Account'
import DateInput from './DateInput.vue'

const props = defineProps<{ account?: Account }>()
const emit = defineEmits<{ saved: [] }>()

const store = useAccountsStore()
const toast = useToast()
const isEdit = !!props.account

const form = reactive({
  name: props.account?.name ?? '',
  type: (props.account?.type ?? 'checking') as AccountType,
  institution: props.account?.institution ?? '',
  includeInFireCalculations: props.account?.includeInFireCalculations ?? true,
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
    toast.add({ title: 'Account updated', color: 'success' })
  } else {
    await store.create(payload)
    toast.add({ title: 'Account created', color: 'success' })
  }
  emit('saved')
}
</script>

<template>
  <UForm :state="form" @submit="onSubmit" class="space-y-4">
    <UFormField label="Name" required>
      <UInput v-model="form.name" class="w-full" placeholder="e.g. Fidelity Brokerage" />
    </UFormField>

    <UFormField label="Type" required>
      <USelect
        class="w-full"
        v-model="form.type"
        :items="accountTypeItems"
        value-key="value"
        placeholder="Select account type"
      />
    </UFormField>

    <UFormField label="Institution (optional)">
      <UInput v-model="form.institution" class="w-full" placeholder="e.g. Fidelity" />
    </UFormField>
    
    <UFormField label="Opened">
      <DateInput v-model="form.createdAt" class="w-full" />
    </UFormField>
    
    <div class="pt-1.5">
      <div class="flex items-center justify-between rounded-lg border border-default px-4 py-3">
        <span class="text-sm font-medium">Include in FIRE calculations</span>
        <USwitch v-model="form.includeInFireCalculations" />
      </div>
    </div>
    
    <div class="pt-4 sm:pt-6 flex justify-end items-center gap-3">
      <UButton type="submit" :disabled="!form.name">
        {{ isEdit ? 'Save Changes' : 'Add Account' }}
      </UButton>
    </div>
  </UForm>
</template>
