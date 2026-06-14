<script setup lang="ts">
import { ref } from 'vue'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useAccountsStore } from '../stores/accounts'
import type { AccountBalance } from '../lib/types/AccountBalance'
import DateInput from './DateInput.vue'
import CurrencyInput from './CurrencyInput.vue'

const props = defineProps<{ balance: AccountBalance }>()
const emit = defineEmits<{ (e: 'view-transaction', id: number): void }>()

const store = useAccountsStore()

const isEditing = ref(false)
const draftBalance = ref<number>(props.balance.balance)
const draftDate = ref<string>(props.balance.recordedAt)

function startEdit() {
  draftBalance.value = props.balance.balance
  draftDate.value = props.balance.recordedAt
  isEditing.value = true
}

function cancelEdit() {
  isEditing.value = false
}

async function save() {
  await store.updateBalanceSnapshot({
    id: props.balance.id,
    balance: draftBalance.value ?? 0,
    recordedAt: draftDate.value,
  })
  isEditing.value = false
}

async function remove() {
  const ok = await confirm(
    `Delete this balance snapshot from ${props.balance.recordedAt}? This cannot be undone.`,
    { title: 'Delete Snapshot?', kind: 'warning' },
  )
  if (ok) await store.removeBalanceSnapshot(props.balance.id)
}

const formatted = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
</script>

<template>
  <tr class="border-b border-gray-100 last:border-0">
    <template v-if="isEditing">
      <td class="py-1 pr-6">
        <DateInput v-model="draftDate" />
      </td>
      <td class="py-1 text-right">
        <CurrencyInput v-model="draftBalance" class="w-32" />
      </td>
      <td class="py-1 text-right">
        <div class="flex justify-end gap-1">
          <UButton size="xs" variant="ghost" @click="save">Save</UButton>
          <UButton size="xs" variant="ghost" color="neutral" @click="cancelEdit">Cancel</UButton>
        </div>
      </td>
    </template>
    <template v-else>
      <td class="py-1 pr-6 text-gray-600">{{ balance.recordedAt }}</td>
      <td class="py-1 text-right font-mono">{{ formatted(balance.balance) }}</td>
      <td class="py-1 text-right">
        <div class="flex justify-end gap-1">
          <UButton
            v-if="balance.linkedTransactionId != null"
            size="xs"
            variant="ghost"
            icon="i-ph-receipt"
            title="View linked transaction"
            @click="emit('view-transaction', balance.linkedTransactionId)"
          />
          <UButton size="xs" variant="ghost" @click="startEdit">Edit</UButton>
          <UButton size="xs" variant="ghost" color="error" @click="remove">Delete</UButton>
        </div>
      </td>
    </template>
  </tr>
</template>
