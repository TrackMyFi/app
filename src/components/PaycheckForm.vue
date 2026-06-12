<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { DateTime } from 'luxon'
import { usePaychecksStore } from '../stores/paychecks'
import { useAccountsStore } from '../stores/accounts'
import { contributionItems } from '../lib/paychecks/index'
import { INVESTMENT_TYPES } from '../lib/accountTypes'
import DateInput from './DateInput.vue'
import type { Paycheck } from '../lib/types/Paycheck'

const PAY_PERIODS = ['weekly', 'biweekly', 'semimonthly', 'monthly', 'irregular'] as const

const props = defineProps<{ editing: Paycheck | null }>()
const emit = defineEmits<{ saved: [] }>()

const store = usePaychecksStore()
const accountsStore = useAccountsStore()

const today = DateTime.now().toISODate()!

interface DeductionRow {
  label: string
  amount: number
  preTax: boolean
  contributionAccountType: string | null
  accountId: number | null
}

interface MatchRow {
  label: string
  amount: number
  accountId: number | null
}

const form = reactive({
  payDate: today,
  employer: '',
  payPeriod: 'biweekly',
  grossAmount: 0,
  netAmount: 0,
  federalTax: 0,
  stateTax: 0,
  localTax: 0,
  socialSecurityTax: 0,
  medicareTax: 0,
  deductions: [] as DeductionRow[],
  employerMatch: [] as MatchRow[],
})

function resetForm() {
  form.payDate = today
  form.employer = ''
  form.payPeriod = 'biweekly'
  form.grossAmount = 0
  form.netAmount = 0
  form.federalTax = 0
  form.stateTax = 0
  form.localTax = 0
  form.socialSecurityTax = 0
  form.medicareTax = 0
  form.deductions = []
  form.employerMatch = []
}

watch(
  () => props.editing,
  (e) => {
    if (e) {
      form.payDate = e.payDate
      form.employer = e.employer
      form.payPeriod = e.payPeriod
      form.grossAmount = e.grossAmount
      form.netAmount = e.netAmount
      form.federalTax = e.federalTax
      form.stateTax = e.stateTax
      form.localTax = e.localTax
      form.socialSecurityTax = e.socialSecurityTax
      form.medicareTax = e.medicareTax
      form.deductions = e.deductions.map((d) => ({
        label: d.label,
        amount: d.amount,
        preTax: d.preTax,
        contributionAccountType: d.contributionAccountType ?? null,
        accountId: d.accountId ?? null,
      }))
      form.employerMatch = e.employerMatch.map((m) => ({
        label: m.label,
        amount: m.amount,
        accountId: m.accountId ?? null,
      }))
    } else {
      resetForm()
    }
  },
  { immediate: true },
)

function accountsForType(type: string | null) {
  if (!type) return []
  return accountsStore.accounts
    .filter((a) => a.type === type && a.isActive)
    .map((a) => ({ label: a.name, value: a.id }))
}

const investmentAccountItems = computed(() =>
  accountsStore.accounts
    .filter((a) => INVESTMENT_TYPES.has(a.type) && a.isActive)
    .map((a) => ({ label: a.name, value: a.id })),
)

function addDeduction() {
  form.deductions.push({ label: '', amount: 0, preTax: true, contributionAccountType: null, accountId: null })
}
function removeDeduction(i: number) { form.deductions.splice(i, 1) }

function addMatch() {
  form.employerMatch.push({ label: '', amount: 0, accountId: null })
}
function removeMatch(i: number) { form.employerMatch.splice(i, 1) }

function onContributionTypeChange(ded: DeductionRow) {
  ded.accountId = null
}

const preview = computed(() => {
  const items = contributionItems(form.deductions, form.employerMatch)
  return items.map((item) => ({
    ...item,
    accountName: accountsStore.accounts.find((a) => a.id === item.accountId)?.name ?? `#${item.accountId}`,
  }))
})

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

async function save() {
  const now = DateTime.now().toISO()!
  if (props.editing) {
    await store.update({
      id: props.editing.id,
      payDate: form.payDate,
      employer: form.employer,
      payPeriod: form.payPeriod,
      grossAmount: form.grossAmount,
      netAmount: form.netAmount,
      federalTax: form.federalTax,
      stateTax: form.stateTax,
      localTax: form.localTax,
      socialSecurityTax: form.socialSecurityTax,
      medicareTax: form.medicareTax,
      deductions: form.deductions,
      employerMatch: form.employerMatch,
      updatedAt: now,
    })
  } else {
    await store.create({
      payDate: form.payDate,
      employer: form.employer,
      payPeriod: form.payPeriod,
      grossAmount: form.grossAmount,
      netAmount: form.netAmount,
      federalTax: form.federalTax,
      stateTax: form.stateTax,
      localTax: form.localTax,
      socialSecurityTax: form.socialSecurityTax,
      medicareTax: form.medicareTax,
      deductions: form.deductions,
      employerMatch: form.employerMatch,
      createdAt: now,
    })
  }
  emit('saved')
}
</script>

<template>
  <form class="space-y-5" @submit.prevent="save">

    <!-- Paycheck info -->
    <div class="space-y-3">
      <p class="text-xs font-semibold uppercase tracking-wide text-muted">Paycheck info</p>
      <div class="grid grid-cols-3 gap-3">
        <div>
          <p class="text-xs text-muted mb-1">Pay date</p>
          <DateInput v-model="form.payDate" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Employer</p>
          <UInput v-model="form.employer" placeholder="Employer" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Pay period</p>
          <USelect
            v-model="form.payPeriod"
            :items="PAY_PERIODS.map((p) => ({ label: p, value: p }))"
          />
        </div>
      </div>
    </div>

    <!-- Amounts -->
    <div class="space-y-3">
      <p class="text-xs font-semibold uppercase tracking-wide text-muted">Amounts</p>
      <div class="grid grid-cols-2 gap-3">
        <div>
          <p class="text-xs text-muted mb-1">Gross</p>
          <UInput v-model.number="form.grossAmount" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Net (take-home)</p>
          <UInput v-model.number="form.netAmount" type="number" step="0.01" placeholder="0.00" />
        </div>
      </div>
    </div>

    <!-- Taxes -->
    <div class="space-y-3">
      <p class="text-xs font-semibold uppercase tracking-wide text-muted">Taxes withheld</p>
      <div class="grid grid-cols-3 gap-3">
        <div>
          <p class="text-xs text-muted mb-1">Federal</p>
          <UInput v-model.number="form.federalTax" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">State</p>
          <UInput v-model.number="form.stateTax" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Local</p>
          <UInput v-model.number="form.localTax" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Social Security</p>
          <UInput v-model.number="form.socialSecurityTax" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Medicare</p>
          <UInput v-model.number="form.medicareTax" type="number" step="0.01" placeholder="0.00" />
        </div>
      </div>
    </div>

    <!-- Deductions -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <p class="text-xs font-semibold uppercase tracking-wide text-muted">Deductions</p>
        <UButton size="xs" variant="soft" icon="i-lucide-plus" @click="addDeduction">Add</UButton>
      </div>
      <div v-for="(ded, i) in form.deductions" :key="i" class="rounded border border-default p-3 space-y-2">
        <div class="grid grid-cols-[1fr_auto_auto_auto] gap-2 items-center">
          <UInput v-model="ded.label" placeholder="Label" />
          <UInput v-model.number="ded.amount" type="number" step="0.01" placeholder="0.00" class="w-28" />
          <UCheckbox v-model="ded.preTax" label="Pre-tax" />
          <UButton size="xs" variant="ghost" color="error" icon="i-lucide-x" @click="removeDeduction(i)" />
        </div>
        <div class="grid grid-cols-2 gap-2">
          <div>
            <p class="text-xs text-muted mb-1">Contribution type (optional)</p>
            <USelect
              v-model="ded.contributionAccountType"
              :items="[{ label: '401k', value: '401k' }, { label: 'roth_401k', value: 'roth_401k' }, { label: 'traditional_ira', value: 'traditional_ira' }, { label: 'roth_ira', value: 'roth_ira' }, { label: 'hsa', value: 'hsa' }, { label: 'brokerage', value: 'brokerage' }, { label: 'crypto', value: 'crypto' }]"
              placeholder="None"
              @update:model-value="onContributionTypeChange(ded)"
            />
          </div>
          <div v-if="ded.contributionAccountType">
            <p class="text-xs text-muted mb-1">Account</p>
            <USelect
              v-model="ded.accountId"
              :items="accountsForType(ded.contributionAccountType)"
              placeholder="Select account"
            />
          </div>
        </div>
      </div>
      <p v-if="form.deductions.length === 0" class="text-xs text-muted">No deductions added.</p>
    </div>

    <!-- Employer Match -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <p class="text-xs font-semibold uppercase tracking-wide text-muted">Employer Match</p>
        <UButton size="xs" variant="soft" icon="i-lucide-plus" @click="addMatch">Add</UButton>
      </div>
      <div v-for="(em, i) in form.employerMatch" :key="i" class="grid grid-cols-[1fr_auto_1fr_auto] gap-2 items-center">
        <UInput v-model="em.label" placeholder="Label" />
        <UInput v-model.number="em.amount" type="number" step="0.01" placeholder="0.00" class="w-28" />
        <USelect v-model="em.accountId" :items="investmentAccountItems" placeholder="Account" />
        <UButton size="xs" variant="ghost" color="error" icon="i-lucide-x" @click="removeMatch(i)" />
      </div>
      <p v-if="form.employerMatch.length === 0" class="text-xs text-muted">No employer match added.</p>
    </div>

    <!-- Contribution preview -->
    <div v-if="preview.length > 0" class="rounded border border-default p-3 space-y-1">
      <p class="text-xs font-semibold uppercase tracking-wide text-muted">Contributions that will be created</p>
      <div v-for="item in preview" :key="item.accountId + item.label" class="flex justify-between text-sm">
        <span class="text-muted">{{ item.label }} → {{ item.accountName }}</span>
        <span class="tabular-nums text-green-600">{{ money(item.amount) }}</span>
      </div>
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <UButton type="submit">{{ props.editing ? 'Save' : 'Add paycheck' }}</UButton>
    </div>

  </form>
</template>
