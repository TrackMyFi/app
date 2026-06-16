<script setup lang="ts">
import { ref } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { useAccountsStore } from '../stores/accounts'
import DateInput from './DateInput.vue'
import CurrencyInput from './CurrencyInput.vue'

const props = defineProps<{ accountId: number }>()

const store = useAccountsStore()
const toast = useToast()

const balance = ref<number>(0)
const recordedAt = ref<string>(DateTime.now().toISODate()!)

async function onSubmit() {
  await store.addBalanceSnapshot({
    accountId: props.accountId,
    balance: balance.value ?? 0,
    recordedAt: recordedAt.value,
  })
  toast.add({ title: 'Balance recorded', color: 'success' })
  balance.value = 0
  recordedAt.value = DateTime.now().toISODate()!
}
</script>

<template>
  <UForm :state="{ balance, recordedAt }" @submit="onSubmit" class="flex items-end gap-2">
    <UFormField label="Balance">
      <CurrencyInput v-model="balance" class="w-36" />
    </UFormField>
    <UFormField label="Date">
      <DateInput v-model="recordedAt" />
    </UFormField>
    <UButton type="submit" size="sm" class="mb-0.5">Add Snapshot</UButton>
  </UForm>
</template>
