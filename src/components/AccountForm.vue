<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { useAccountsStore } from '../stores/accounts'
import { accountTypeItems, defaultIncludeInFire, defaultCountPaymentsAsExpense, isLiability, type AccountType } from '../lib/accountTypes'
import type { Account } from '../lib/types/Account'
import DateInput from './DateInput.vue'

const props = defineProps<{ account?: Account }>()
const emit = defineEmits<{ saved: [] }>()

const store = useAccountsStore()
const toast = useToast()
const isEdit = !!props.account
const saving = ref(false)

const form = reactive({
  name: props.account?.name ?? '',
  type: (props.account?.type ?? 'checking') as AccountType,
  institution: props.account?.institution ?? '',
  includeInFireCalculations: props.account?.includeInFireCalculations ?? true,
  countPaymentsAsExpense: props.account?.countPaymentsAsExpense ?? false,
  createdAt: props.account?.createdAt ?? DateTime.now().toISODate()!,
})

// Auto-default the FIRE and payment-as-expense toggles from the account type
// ONLY when adding, so editing an existing account never silently flips a
// user's stored choice.
if (!isEdit) {
  watch(
    () => form.type,
    (newType) => {
      form.includeInFireCalculations = defaultIncludeInFire(newType)
      form.countPaymentsAsExpense = defaultCountPaymentsAsExpense(newType)
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
    countPaymentsAsExpense: isLiability(form.type) && form.countPaymentsAsExpense,
    createdAt: form.createdAt,
  }
  saving.value = true
  try {
    if (isEdit) {
      await store.update(props.account!.id, payload)
      toast.add({ title: 'Account updated', color: 'success' })
    } else {
      await store.create(payload)
      toast.add({ title: 'Account created', color: 'success' })
    }
    emit('saved')
  } catch (err) {
    toast.add({ title: 'Failed to save account', description: String(err), color: 'error' })
  } finally {
    saving.value = false
  }
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
    
    <div class="pt-1.5 space-y-3">
      <div class="flex items-center justify-between rounded-lg border border-default px-4 py-3">
        <span class="text-sm font-medium">Include in FIRE calculations</span>
        <USwitch v-model="form.includeInFireCalculations" />
      </div>
      <div v-if="isLiability(form.type)" class="rounded-lg border border-default px-4 py-3">
        <div class="flex items-center justify-between">
          <span class="text-sm font-medium">Count payments as expenses</span>
          <USwitch v-model="form.countPaymentsAsExpense" />
        </div>
        <p class="text-xs text-muted mt-1.5">
          For loans and mortgages where no purchases are tracked — transfers into
          this account count as fixed spending. Leave off for credit cards, whose
          purchases are already recorded.
        </p>
      </div>
    </div>
    
    <div class="pt-4 flex justify-end items-center gap-3">
      <UButton type="submit" :loading="saving" :disabled="!form.name || saving" block>
        {{ isEdit ? 'Save Changes' : 'Add Account' }}
      </UButton>
    </div>
  </UForm>
</template>
