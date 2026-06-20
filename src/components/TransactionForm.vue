<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { useTransactionsStore } from '../stores/transactions'
import { useAccountsStore } from '../stores/accounts'
import { transactionTypeItems, categoryItems } from '../lib/transactions/constants'
import { balancePreview } from '../lib/transactions/balancePreview'
import { isInvestment, isLiability } from '../lib/accountTypes'
import DateInput from './DateInput.vue'
import type { Transaction } from '../lib/types/Transaction'

const props = defineProps<{ editing: Transaction | null }>()
const emit = defineEmits<{ saved: [] }>()

const store = useTransactionsStore()
const accountsStore = useAccountsStore()
const toast = useToast()

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

const isContributionTransfer = computed(() => {
  if (!isTransfer.value || form.accountId == null || form.transferAccountId == null) return false
  const src = accountsStore.accounts.find((a) => a.id === form.accountId)
  const dst = accountsStore.accounts.find((a) => a.id === form.transferAccountId)
  return src != null && dst != null && !isInvestment(src.type) && isInvestment(dst.type)
})

watch([() => form.accountId, () => form.transferAccountId], () => {
  if (!props.editing) form.isContribution = isContributionTransfer.value
})

// Default the switch on for cash/liability accounts, off for investment accounts.
function defaultUpdateBalance(accountId: number | undefined): boolean {
  if (accountId == null) return false
  const acct = accountsStore.accounts.find((a) => a.id === accountId)
  return acct ? !isInvestment(acct.type) : false
}
const updateBalance = ref(false)
watch(() => form.accountId, (id) => { updateBalance.value = defaultUpdateBalance(id) })

const liabilityIds = computed(
  () => new Set(accountsStore.accounts.filter((a) => isLiability(a.type)).map((a) => a.id)),
)

const preview = computed(() =>
  form.accountId == null
    ? []
    : balancePreview(
        accountsStore.allBalances,
        {
          type: form.type,
          amount: form.amount || 0,
          accountId: form.accountId,
          transferAccountId: form.transferAccountId,
          date: form.date,
        },
        liabilityIds.value,
      ),
)

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}
function accountName(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

async function save() {
  if (form.accountId == null) return
  const now = DateTime.now().toISO()!
  if (props.editing) {
    await store.update({
      id: props.editing.id,
      accountId: form.accountId,
      transferAccountId: isTransfer.value ? form.transferAccountId : null,
      amount: form.amount ?? 0,
      description: form.description,
      date: form.date,
      type: form.type,
      category: form.category,
      isContribution: form.isContribution,
      updateBalance: updateBalance.value,
      updatedAt: now,
    })
    toast.add({ title: 'Transaction updated', color: 'success' })
  } else {
    await store.create({
      accountId: form.accountId,
      transferAccountId: isTransfer.value ? form.transferAccountId : null,
      amount: form.amount ?? 0,
      description: form.description,
      date: form.date,
      type: form.type,
      category: form.category,
      isContribution: form.isContribution,
      importSource: 'manual',
      updateBalance: updateBalance.value,
      createdAt: now,
    })
    toast.add({ title: 'Transaction added', color: 'success' })
  }
  emit('saved')
}
</script>

<template>
  <form class="space-y-4" @submit.prevent="save">
    <UFormField label="Type">
      <USelect v-model="form.type" :items="transactionTypeItems" class="w-full" />
    </UFormField>
    <div class="grid grid-cols-2 gap-3">
      <UFormField :label="isTransfer ? 'From account' : 'Account'" :class="{ 'col-span-full': !isTransfer }">
        <USelect v-model="form.accountId" :items="accountItems" placeholder="Select account" class="w-full" />
      </UFormField>
      <UFormField v-if="isTransfer" label="To account">
        <USelect v-model="form.transferAccountId" :items="accountItems" placeholder="Select account" class="w-full" />
      </UFormField>
    </div>
    <UFormField label="Amount">
      <CurrencyInput v-model="form.amount" class="w-full" />
    </UFormField>
    <UFormField label="Date">
      <DateInput v-model="form.date" class="w-full" />
    </UFormField>
    <UFormField label="Description">
      <UInput v-model="form.description" placeholder="Optional" class="w-full" />
    </UFormField>
    <UFormField v-if="!isTransfer" label="Category">
      <USelect v-model="form.category" :items="categoryItems" class="w-full" />
    </UFormField>
    <UCheckbox v-if="!isTransfer || isContributionTransfer" v-model="form.isContribution" label="Counts as an investment contribution" />

    <div class="rounded-lg border border-default p-3 space-y-2">
      <USwitch v-model="updateBalance" label="Update account balance" />
      <p class="text-xs text-muted">
        Writes a new balance snapshot reflecting this transaction, so the change shows up in your
        net-worth history. Leave off to record the transaction without touching balances.
      </p>
      <div v-if="updateBalance" class="text-sm space-y-1">
        <div v-for="line in preview" :key="line.accountId" class="tabular-nums">
          {{ accountName(line.accountId) }}: {{ money(line.from) }} → <strong>{{ money(line.to) }}</strong>
        </div>
      </div>
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <UButton type="submit">{{ props.editing ? 'Save' : 'Add' }} Transaction</UButton>
    </div>
  </form>
</template>
