<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { usePaychecksStore } from '../stores/paychecks'
import { useAccountsStore } from '../stores/accounts'
import { contributionItems } from '../lib/paychecks/index'
import { INVESTMENT_TYPES, investmentTypeItems, isLiability } from '../lib/accountTypes'
import { payPeriodItems } from '../lib/paychecks/constants'
import DateInput from './DateInput.vue'
import ComboboxInput from './ComboboxInput.vue'
import CurrencyInput from './CurrencyInput.vue'
import type { Paycheck } from '../lib/types/Paycheck'


const props = defineProps<{ editing: Paycheck | null; copyFrom: Paycheck | null }>()
const emit = defineEmits<{ saved: [] }>()

const store = usePaychecksStore()
const accountsStore = useAccountsStore()
const toast = useToast()
const saving = ref(false)
const saveError = ref<string | null>(null)

const knownEmployers = computed(() =>
  [...new Set(store.paychecks.map((p) => p.employer).filter(Boolean))]
)

const knownDeductionLabels = computed(() =>
  [...new Set(store.paychecks.flatMap((p) => p.deductions.map((d) => d.label)).filter(Boolean))]
)

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
  incomeAccountId: null as number | null,
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
  form.incomeAccountId = null
  form.federalTax = 0
  form.stateTax = 0
  form.localTax = 0
  form.socialSecurityTax = 0
  form.medicareTax = 0
  form.deductions = []
  form.employerMatch = []
}

watch(
  () => [props.editing, props.copyFrom] as const,
  ([e, c]) => {
    saveError.value = null
    if (e) {
      form.payDate = e.payDate
      form.employer = e.employer
      form.payPeriod = e.payPeriod
      form.grossAmount = e.grossAmount
      form.netAmount = e.netAmount
      form.incomeAccountId = e.incomeAccountId ?? null
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
    } else if (c) {
      form.payDate = DateTime.now().toISODate()!
      form.employer = c.employer
      form.payPeriod = c.payPeriod
      form.grossAmount = c.grossAmount
      form.netAmount = c.netAmount
      form.incomeAccountId = c.incomeAccountId ?? null
      form.federalTax = c.federalTax
      form.stateTax = c.stateTax
      form.localTax = c.localTax
      form.socialSecurityTax = c.socialSecurityTax
      form.medicareTax = c.medicareTax
      form.deductions = c.deductions.map((d) => ({
        label: d.label,
        amount: d.amount,
        preTax: d.preTax,
        contributionAccountType: d.contributionAccountType ?? null,
        accountId: d.accountId ?? null,
      }))
      form.employerMatch = c.employerMatch.map((m) => ({
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

const depositAccountItems = computed(() =>
  accountsStore.accounts
    .filter((a) => !INVESTMENT_TYPES.has(a.type) && !isLiability(a.type) && a.isActive)
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
  saveError.value = null
  const now = DateTime.now().toISO()!
  saving.value = true
  try {
    if (props.editing) {
      await store.update({
        id: props.editing.id,
        payDate: form.payDate,
        employer: form.employer,
        payPeriod: form.payPeriod,
        grossAmount: form.grossAmount ?? 0,
        netAmount: form.netAmount ?? 0,
        federalTax: form.federalTax ?? 0,
        stateTax: form.stateTax ?? 0,
        localTax: form.localTax ?? 0,
        socialSecurityTax: form.socialSecurityTax ?? 0,
        medicareTax: form.medicareTax ?? 0,
        deductions: form.deductions,
        employerMatch: form.employerMatch,
        incomeAccountId: form.incomeAccountId,
        updatedAt: now,
      })
      toast.add({ title: 'Paycheck updated', color: 'success' })
    } else {
      await store.create({
        payDate: form.payDate,
        employer: form.employer,
        payPeriod: form.payPeriod,
        grossAmount: form.grossAmount ?? 0,
        netAmount: form.netAmount ?? 0,
        federalTax: form.federalTax ?? 0,
        stateTax: form.stateTax ?? 0,
        localTax: form.localTax ?? 0,
        socialSecurityTax: form.socialSecurityTax ?? 0,
        medicareTax: form.medicareTax ?? 0,
        deductions: form.deductions,
        employerMatch: form.employerMatch,
        incomeAccountId: form.incomeAccountId,
        createdAt: now,
      })
      toast.add({ title: 'Paycheck added', color: 'success' })
    }
    emit('saved')
  } catch (err) {
    saveError.value = String(err)
    toast.add({ title: 'Failed to save paycheck', description: String(err), color: 'error' })
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <form class="space-y-5" @submit.prevent="save">

    <div class="flex flex-wrap lg:flex-nowrap gap-3 space-y-5">
      <div class="flex-1 space-y-5">
        <!-- Paycheck info -->
        <div class="space-y-3">
          <p class="text-xs font-semibold uppercase tracking-wide text-muted">Paycheck info</p>
          <div class="flex gap-3">
            <div class="flex-1">
              <p class="text-xs text-muted mb-1">Pay date</p>
              <DateInput v-model="form.payDate" class="w-full" />
            </div>
            <div class="w-42">
              <p class="text-xs text-muted mb-1">Pay period</p>
              <USelect
                v-model="form.payPeriod"
                :items="payPeriodItems"
                class="w-full"
              />
            </div>
          </div>
          <div>
            <p class="text-xs text-muted mb-1">Employer</p>
            <ComboboxInput v-model="form.employer" :items="knownEmployers" placeholder="Employer" class="w-full" />
          </div>
          <div>
            <p class="text-xs text-muted mb-1">Deposit to account</p>
            <USelect
              v-model="form.incomeAccountId"
              :items="depositAccountItems"
              placeholder="None (optional)"
              class="w-full"
            />
          </div>
        </div>

        <!-- Amounts -->
        <div class="space-y-3">
          <p class="text-xs font-semibold uppercase tracking-wide text-muted">Amounts</p>
          <div>
            <p class="text-xs text-muted mb-1">Gross</p>
            <CurrencyInput v-model="form.grossAmount" class="w-full" />
          </div>
          <div>
            <p class="text-xs text-muted mb-1">Net (take-home)</p>
            <CurrencyInput v-model="form.netAmount" class="w-full" />
          </div>
        </div>

        <!-- Taxes -->
        <div class="space-y-3">
          <p class="text-xs font-semibold uppercase tracking-wide text-muted">Taxes withheld</p>
          <div class="grid grid-cols-3 gap-3">
            <div>
              <p class="text-xs text-muted mb-1">Federal</p>
              <CurrencyInput v-model="form.federalTax" />
            </div>
            <div>
              <p class="text-xs text-muted mb-1">State</p>
              <CurrencyInput v-model="form.stateTax" />
            </div>
            <div>
              <p class="text-xs text-muted mb-1">Local</p>
              <CurrencyInput v-model="form.localTax" />
            </div>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <p class="text-xs text-muted mb-1">Social Security</p>
              <CurrencyInput v-model="form.socialSecurityTax" class="w-full" />
            </div>
            <div>
              <p class="text-xs text-muted mb-1">Medicare</p>
              <CurrencyInput v-model="form.medicareTax" class="w-full" />
            </div>
          </div>
        </div>
      </div>

      <div class="w-full lg:w-1/2 bg-muted border border-default/50 rounded-xl space-y-5 lg:-my-4 lg:ml-3 p-4">
        <!-- Deductions -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Deductions</p>
            <UButton size="xs" variant="soft" icon="i-ph-plus" @click="addDeduction">Add</UButton>
          </div>
          <div v-for="(ded, i) in form.deductions" :key="i" class="rounded border border-default p-3 space-y-2">
            <div class="flex gap-2 items-center">
              <ComboboxInput v-model="ded.label" :items="knownDeductionLabels" placeholder="Label" class="flex-1 min-w-0" />
              <CurrencyInput v-model="ded.amount" class="w-24" />
              <UButton size="xs" variant="ghost" color="error" icon="i-ph-x" @click="removeDeduction(i)" />
            </div>
            <div class="flex gap-2 items-center">
              <USelect
                v-model="ded.contributionAccountType"
                :items="investmentTypeItems"
                placeholder="Contribution type"
                class="flex-1"
                @update:model-value="onContributionTypeChange(ded)"
              />
              <USelect
                v-if="ded.contributionAccountType"
                v-model="ded.accountId"
                :items="accountsForType(ded.contributionAccountType)"
                placeholder="Account"
                class="flex-1"
              />
              <UCheckbox v-model="ded.preTax" label="Pre-tax" class="shrink-0" />
            </div>
          </div>
          <p v-if="form.deductions.length === 0" class="text-xs text-muted">No deductions added.</p>
        </div>

        <!-- Employer Match -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Employer Match</p>
            <UButton size="xs" variant="soft" icon="i-ph-plus" @click="addMatch">Add</UButton>
          </div>
          <div v-for="(em, i) in form.employerMatch" :key="i" class="grid grid-cols-[1fr_auto_1fr_auto] gap-2 items-center">
            <UInput v-model="em.label" placeholder="Label" />
            <CurrencyInput v-model="em.amount" class="w-28" />
            <USelect v-model="em.accountId" :items="investmentAccountItems" placeholder="Account" />
            <UButton size="xs" variant="ghost" color="error" icon="i-ph-x" @click="removeMatch(i)" />
          </div>
          <p v-if="form.employerMatch.length === 0" class="text-xs text-muted">No employer match added.</p>
        </div>

        <!-- Contribution preview -->
        <div v-if="preview.length > 0" class="rounded border border-default p-3 space-y-1">
          <p class="text-xs font-semibold uppercase tracking-wide text-muted">Contributions that will be created</p>
          <div v-for="item in preview" :key="`${item.accountId}:${item.label}`" class="flex justify-between text-sm">
            <span class="text-muted">{{ item.label }} → {{ item.accountName }}</span>
            <span class="tabular-nums text-success">{{ money(item.amount) }}</span>
          </div>
        </div>
      </div>
    </div>

    <p v-if="saveError" class="text-sm text-error">{{ saveError }}</p>

    <div class="flex justify-end gap-2 pt-2">
      <UButton type="submit" :loading="saving" :disabled="saving">{{ props.editing ? 'Save' : props.copyFrom ? 'Copy paycheck' : 'Add paycheck' }}</UButton>
    </div>

  </form>
</template>
