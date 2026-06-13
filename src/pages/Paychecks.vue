<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { usePaychecksStore } from '../stores/paychecks'
import { useAccountsStore } from '../stores/accounts'
import { paycheckTotals } from '../lib/paychecks/index'
import PaycheckForm from '../components/PaycheckForm.vue'
import DateInput from '../components/DateInput.vue'
import type { Paycheck } from '../lib/types/Paycheck'
import { confirm } from '@tauri-apps/plugin-dialog'

const store = usePaychecksStore()
const accountsStore = useAccountsStore()

const isModalOpen = ref(false)
const editing = ref<Paycheck | null>(null)
const copySource = ref<Paycheck | null>(null)

function openAdd() { editing.value = null; copySource.value = null; isModalOpen.value = true }
function openEdit(p: Paycheck) { editing.value = p; copySource.value = null; isModalOpen.value = true }
function openCopy(p: Paycheck) { editing.value = null; copySource.value = p; isModalOpen.value = true }
function onSaved() { isModalOpen.value = false }

watch(isModalOpen, (open) => {
  if (!open) {
    editing.value = null
    copySource.value = null
  }
})

async function removeRow(p: Paycheck) {
  const ok = await confirm(`Delete paycheck from "${p.employer}" on ${p.payDate}?`, {
    title: 'Delete paycheck',
  })
  if (ok) await store.remove(p.id)
}

const startDate = ref('')
const endDate = ref('')
const employerSearch = ref('')

async function applyFilters() {
  await store.setFilter({
    startDate: startDate.value || null,
    endDate: endDate.value || null,
    employer: employerSearch.value || null,
  })
}

const totals = computed(() => paycheckTotals(store.paychecks))

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

onMounted(async () => {
  await accountsStore.load() // pre-populates store for PaycheckForm account dropdowns
  await store.load()
})
</script>

<template>
  <div class="p-6 space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Paychecks</h1>
      <UButton icon="i-lucide-plus" @click="openAdd">Add paycheck</UButton>
    </div>

    <div class="flex flex-wrap gap-2 items-end">
      <div>
        <p class="text-xs text-muted mb-1">From</p>
        <DateInput v-model="startDate" />
      </div>
      <div>
        <p class="text-xs text-muted mb-1">To</p>
        <DateInput v-model="endDate" />
      </div>
      <UInput v-model="employerSearch" placeholder="Search employer" class="w-44" />
      <UButton @click="applyFilters">Apply</UButton>
    </div>

    <div class="flex gap-6 text-sm">
      <span>Gross: <strong>{{ money(totals.totalGross) }}</strong></span>
      <span>Net: <strong>{{ money(totals.totalNet) }}</strong></span>
      <span class="text-muted">{{ totals.count }} paychecks</span>
    </div>

    <table class="w-full text-sm">
      <thead class="text-left text-muted border-b border-default">
        <tr>
          <th class="py-2">Date</th>
          <th>Employer</th>
          <th>Period</th>
          <th class="text-right">Gross</th>
          <th class="text-right">Net</th>
          <th class="text-right">Federal</th>
          <th class="text-right">SS + Medicare</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="p in store.paychecks" :key="p.id" class="border-b border-default/50">
          <td class="py-2">{{ p.payDate }}</td>
          <td>{{ p.employer }}</td>
          <td class="capitalize">{{ p.payPeriod }}</td>
          <td class="text-right tabular-nums">{{ money(p.grossAmount) }}</td>
          <td class="text-right tabular-nums">{{ money(p.netAmount) }}</td>
          <td class="text-right tabular-nums">{{ money(p.federalTax) }}</td>
          <td class="text-right tabular-nums">{{ money(p.socialSecurityTax + p.medicareTax) }}</td>
          <td class="text-right">
            <UButton size="xs" variant="ghost" icon="i-lucide-copy" @click="openCopy(p)" />
            <UButton size="xs" variant="ghost" icon="i-lucide-pencil" @click="openEdit(p)" />
            <UButton size="xs" variant="ghost" color="error" icon="i-lucide-trash-2" @click="removeRow(p)" />
          </td>
        </tr>
        <tr v-if="!store.paychecks.length">
          <td colspan="8" class="py-6 text-center text-muted">No paychecks yet.</td>
        </tr>
      </tbody>
    </table>

    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit paycheck' : copySource ? 'Copy paycheck' : 'Add paycheck'">
      <template #body>
        <PaycheckForm :editing="editing" :copy-from="copySource" @saved="onSaved" />
      </template>
    </UModal>
  </div>
</template>
