<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { usePaychecksStore } from '../stores/paychecks'
import { labelForPayPeriod } from '../lib/paychecks/constants'
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

async function clearFilters() {
  startDate.value = ''
  endDate.value = ''
  employerSearch.value = ''
  await store.setFilter({ startDate: null, endDate: null, employer: null })
}

const totals = computed(() => paycheckTotals(store.paychecks))

const columns = [
  { accessorKey: 'payDate', header: 'Date' },
  { accessorKey: 'employer', header: 'Employer' },
  { id: 'period', header: 'Period' },
  { id: 'gross', header: 'Gross', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'net', header: 'Net', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'federal', header: 'Federal', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'ssMedicare', header: 'SS + Medicare', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  { id: 'actions', header: '', meta: { class: { td: 'text-right' } } },
]

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

onMounted(async () => {
  await accountsStore.load() // pre-populates store for PaycheckForm account dropdowns
  await store.load()
  startDate.value = store.filter.startDate ?? ''
  endDate.value = store.filter.endDate ?? ''
  employerSearch.value = store.filter.employer ?? ''
})
</script>

<template>
  <div class="p-6 space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Paychecks</h1>
      <UButton icon="i-ph-plus" @click="openAdd">Add paycheck</UButton>
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
      <UButton variant="ghost" @click="clearFilters">Clear</UButton>
    </div>

    <div class="flex gap-6 text-sm">
      <span>Gross: <strong>{{ money(totals.totalGross) }}</strong></span>
      <span>Net: <strong>{{ money(totals.totalNet) }}</strong></span>
      <span class="text-muted">{{ totals.count }} paychecks</span>
    </div>

    <div class="border border-default rounded-lg overflow-hidden">
      <UTable :data="store.paychecks" :columns="columns" empty="No paychecks yet.">
        <template #period-cell="{ row }">{{ labelForPayPeriod(row.original.payPeriod) }}</template>
        <template #gross-cell="{ row }">{{ money(row.original.grossAmount) }}</template>
        <template #net-cell="{ row }">{{ money(row.original.netAmount) }}</template>
        <template #federal-cell="{ row }">{{ money(row.original.federalTax) }}</template>
        <template #ssMedicare-cell="{ row }">{{ money(row.original.socialSecurityTax + row.original.medicareTax) }}</template>
        <template #actions-cell="{ row }">
          <UButton size="xs" variant="ghost" icon="i-ph-copy" @click="openCopy(row.original)" />
          <UButton size="xs" variant="ghost" icon="i-ph-pencil" @click="openEdit(row.original)" />
          <UButton size="xs" variant="ghost" color="error" icon="i-ph-trash" @click="removeRow(row.original)" />
        </template>
      </UTable>
    </div>

    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit paycheck' : copySource ? 'Copy paycheck' : 'Add paycheck'" class="lg:w-[900px] max-w-full">
      <template #body>
        <PaycheckForm :editing="editing" :copy-from="copySource" @saved="onSaved" />
      </template>
    </UModal>
  </div>
</template>
