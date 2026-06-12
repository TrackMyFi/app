<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { DateTime } from 'luxon'
import { useTransactionsStore } from '../stores/transactions'
import { useAccountsStore } from '../stores/accounts'
import { TRANSACTION_TYPES, CATEGORIES } from '../lib/transactions/constants'
import DateInput from './DateInput.vue'
import type { Transaction } from '../lib/types/Transaction'

const props = defineProps<{ editing: Transaction | null }>()
const emit = defineEmits<{ saved: [] }>()

const store = useTransactionsStore()
const accountsStore = useAccountsStore()

const today = DateTime.now().toISODate()!

const form = reactive({
  accountId: undefined as number | undefined,
  transferAccountId: null as number | null,
  amount: 0,
  description: '',
  date: today,
  type: 'expense',
  category: 'uncategorized',
  isContribution: false,
})

watch(
  () => props.editing,
  (e) => {
    if (e) {
      form.accountId = e.accountId
      form.transferAccountId = e.transferAccountId
      form.amount = e.amount
      form.description = e.description
      form.date = e.date
      form.type = e.type
      form.category = e.category
      form.isContribution = e.isContribution
    } else {
      form.accountId = undefined
      form.transferAccountId = null
      form.amount = 0
      form.description = ''
      form.date = today
      form.type = 'expense'
      form.category = 'uncategorized'
      form.isContribution = false
    }
  },
  { immediate: true },
)

const isTransfer = computed(() => form.type === 'transfer')
const accountItems = computed(() =>
  accountsStore.accounts.map((a) => ({ label: a.name, value: a.id })),
)

async function save() {
  if (form.accountId == null) return
  const now = DateTime.now().toISO()!
  if (props.editing) {
    await store.update({
      id: props.editing.id,
      accountId: form.accountId,
      transferAccountId: isTransfer.value ? form.transferAccountId : null,
      amount: form.amount,
      description: form.description,
      date: form.date,
      type: form.type,
      category: form.category,
      isContribution: form.isContribution,
      updateBalance: false,
      updatedAt: now,
    })
  } else {
    await store.create({
      accountId: form.accountId,
      transferAccountId: isTransfer.value ? form.transferAccountId : null,
      amount: form.amount,
      description: form.description,
      date: form.date,
      type: form.type,
      category: form.category,
      isContribution: form.isContribution,
      importSource: 'manual',
      updateBalance: false,
      createdAt: now,
    })
  }
  emit('saved')
}
</script>

<template>
  <form class="space-y-3" @submit.prevent="save">
    <USelect v-model="form.type" :items="TRANSACTION_TYPES.map((t) => ({ label: t, value: t }))" />
    <USelect v-model="form.accountId" :items="accountItems" :placeholder="isTransfer ? 'From account' : 'Account'" />
    <USelect v-if="isTransfer" v-model="form.transferAccountId" :items="accountItems" placeholder="To account" />
    <UInput v-model.number="form.amount" type="number" step="0.01" placeholder="Amount" />
    <UInput v-model="form.description" placeholder="Description" />
    <DateInput v-model="form.date" />
    <USelect v-if="!isTransfer" v-model="form.category" :items="CATEGORIES.map((c) => ({ label: c, value: c }))" />
    <UCheckbox v-if="!isTransfer" v-model="form.isContribution" label="Counts as an investment contribution" />
    <div class="flex justify-end gap-2 pt-2">
      <UButton type="submit">{{ props.editing ? 'Save' : 'Add' }}</UButton>
    </div>
  </form>
</template>
