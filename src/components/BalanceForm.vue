<script setup lang="ts">
import { ref } from 'vue'
import { DateTime } from 'luxon'
import { useAccountsStore } from '../stores/accounts'

const props = defineProps<{ accountId: number }>()

const store = useAccountsStore()

const balance = ref<number>(0)
const recordedAt = ref<string>(DateTime.now().toISODate()!)

async function onSubmit() {
  await store.addBalanceSnapshot({
    accountId: props.accountId,
    balance: balance.value,
    recordedAt: recordedAt.value,
  })
  balance.value = 0
  recordedAt.value = DateTime.now().toISODate()!
}
</script>

<template>
  <UForm :state="{ balance, recordedAt }" @submit="onSubmit" class="flex items-end gap-2">
    <UFormField label="Balance ($)">
      <UInput v-model.number="balance" type="number" step="0.01" placeholder="0.00" class="w-36" />
    </UFormField>
    <UFormField label="Date">
      <UInput v-model="recordedAt" type="date" class="w-40" />
    </UFormField>
    <UButton type="submit" size="sm" class="mb-0.5">Add Snapshot</UButton>
  </UForm>
</template>
