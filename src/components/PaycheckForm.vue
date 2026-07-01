<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { usePaychecksStore } from '../stores/paychecks'
import { useAccountsStore } from '../stores/accounts'
import { contributionItems, findDuplicateDeposit, type ExistingTxnRef } from '../lib/paychecks/index'
import { listTransactions } from '../lib/api/transactions'
import { INVESTMENT_TYPES, isInvestment, isLiability } from '../lib/accountTypes'
import { balancePreview, type PreviewLine } from '../lib/transactions/balancePreview'
import { payPeriodItems } from '../lib/paychecks/constants'
import DateInput from './DateInput.vue'
import ComboboxInput from './ComboboxInput.vue'
import CurrencyInput from './CurrencyInput.vue'
import type { Paycheck } from '../lib/types/Paycheck'
import { InputMenuItem } from '@nuxt/ui'


const props = defineProps<{ editing: Paycheck | null; copyFrom: Paycheck | null }>()
const emit = defineEmits<{ saved: [close: boolean] }>()

const store = usePaychecksStore()
const accountsStore = useAccountsStore()
const toast = useToast()
const saving = ref(false)
const saveError = ref<string | null>(null)

// Declared early: `resetForm` (below) resets these, and it can run
// synchronously during the editing/copyFrom watcher's immediate first fire,
// before later top-level statements in this file have executed.
const existingIncomeTxns = ref<ExistingTxnRef[]>([])
const createDepositTxn = ref(true)

// ─── Save modes (split button) ───────────────────────────────────────────────
// Most paychecks look like the last one, so bulk entry is common. The three
// modes mirror the Add Transaction form:
// 'close' — save and dismiss the modal (default)
// 'add'   — save, reset to a blank form, keep the modal open for the next entry
// 'keep'  — save, keep the current values, keep the modal open
type SaveMode = 'close' | 'add' | 'keep'

const saveModeOptions: { value: SaveMode; label: string }[] = [
  { value: 'close', label: 'Save Paycheck' },
  { value: 'add', label: 'Save & Add Another' },
  { value: 'keep', label: 'Save, Keep Values, & Add Another' },
]

const SAVE_MODE_KEY = 'trackmyfi.paycheckSaveMode'
function loadSaveMode(): SaveMode {
  const v = localStorage.getItem(SAVE_MODE_KEY)
  return v === 'add' || v === 'keep' ? v : 'close'
}
const saveMode = ref<SaveMode>(loadSaveMode())
watch(saveMode, (m) => localStorage.setItem(SAVE_MODE_KEY, m))

const saveModeLabel = computed(
  () => saveModeOptions.find((o) => o.value === saveMode.value)?.label ?? 'Save Paycheck',
)
const saveMenuItems = computed(() => [
  saveModeOptions.map((o) => ({
    label: o.label,
    icon: saveMode.value === o.value ? 'i-ph-check' : undefined,
    onSelect: () => {
      saveMode.value = o.value
      save(o.value)
    },
  })),
])

const knownEmployers = computed(() =>
  [...new Set(store.paychecks.map((p) => p.employer).filter(Boolean))]
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
  createDepositTxn.value = true
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

const investmentAccountItems = computed(() =>
  accountsStore.accounts
    .filter((a) => INVESTMENT_TYPES.has(a.type) && a.isActive)
    .map((a) => ({ label: a.name, value: a.id })),
)

const investmentAccountNames = computed(() =>
  investmentAccountItems.value.map((a) => a.label),
)

const pastFreeTextDeductionLabels = computed(() => {
  const accountNames = new Set(investmentAccountNames.value)
  const seen = new Set<string>()
  for (const p of store.paychecks) {
    for (const d of p.deductions) {
      if (d.accountId == null && d.label && !accountNames.has(d.label)) {
        seen.add(d.label)
      }
    }
  }
  return [...seen]
})

const deductionComboItems = computed((): InputMenuItem[] => {
  const items: InputMenuItem[] = []
  if (investmentAccountNames.value.length > 0) {
    items.push({ label: 'Accounts', type: 'label' })
    items.push(...investmentAccountNames.value)
  }
  if (pastFreeTextDeductionLabels.value.length > 0) {
    items.push({ label: 'Past deductions', type: 'label' })
    items.push(...pastFreeTextDeductionLabels.value)
  }
  return items
})

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

function onDeductionLabelChange(ded: DeductionRow, value: string) {
  ded.label = value
  const acct = accountsStore.accounts.find(
    (a) => INVESTMENT_TYPES.has(a.type) && a.isActive && a.name === value,
  )
  if (acct) {
    ded.accountId = acct.id
    ded.contributionAccountType = acct.type
  } else {
    ded.accountId = null
    ded.contributionAccountType = null
  }
}

// ─── Update account balance ──────────────────────────────────────────────────
function defaultUpdateBalance(accountId: number | null): boolean {
  if (accountId == null) return false
  const acct = accountsStore.accounts.find((a) => a.id === accountId)
  return acct ? !isInvestment(acct.type) : false
}
const updateBalance = ref(false)
watch(() => form.incomeAccountId, (id) => { updateBalance.value = defaultUpdateBalance(id) })

// ─── Duplicate deposit detection ─────────────────────────────────────────────
// If a bank CSV was already imported and contains this paycheck's deposit,
// don't create a second (redundant) income transaction for it. Pre-tax
// contribution transactions are unaffected — they're never checked here.
watch(
  () => form.incomeAccountId,
  async (id) => {
    if (id == null) {
      existingIncomeTxns.value = []
      return
    }
    const page = await listTransactions({ accountIds: [id], limit: 1_000_000 })
    existingIncomeTxns.value = page.rows.map((r) => ({
      id: r.id,
      amount: r.amount,
      date: r.date,
      description: r.description,
      type: r.type,
      paycheckId: r.paycheckId,
    }))
    // Editing an existing paycheck: restore whatever was decided last time
    // (its own deposit txn is excluded from duplicateMatch below, so it can
    // never self-flag) rather than re-defaulting to skip.
    if (props.editing) {
      createDepositTxn.value = existingIncomeTxns.value.some(
        (t) => t.paycheckId === props.editing!.id && t.type === 'income',
      )
    }
  },
  { immediate: true },
)

const duplicateMatch = computed(() =>
  findDuplicateDeposit({ amount: form.netAmount || 0, date: form.payDate }, existingIncomeTxns.value),
)

// New paycheck (not editing): default to skip the redundant deposit the first
// time a match appears, but don't keep clobbering the user's own toggle.
watch(duplicateMatch, (match, prevMatch) => {
  if (match && !prevMatch && !props.editing) createDepositTxn.value = false
})

const liabilityIds = computed(
  () => new Set(accountsStore.accounts.filter((a) => isLiability(a.type)).map((a) => a.id)),
)

const hasLinkedAccounts = computed(
  () => form.incomeAccountId != null || form.deductions.some((d) => d.accountId != null),
)

const allBalancePreviews = computed((): PreviewLine[] => {
  if (!updateBalance.value) return []
  const results: PreviewLine[] = []
  if (form.incomeAccountId != null) {
    results.push(...balancePreview(accountsStore.allBalances, {
      type: 'income', amount: form.netAmount || 0,
      accountId: form.incomeAccountId, transferAccountId: null, date: form.payDate,
    }, liabilityIds.value))
  }
  for (const ded of form.deductions) {
    if (ded.accountId != null) {
      results.push(...balancePreview(accountsStore.allBalances, {
        type: 'income', amount: ded.amount || 0,
        accountId: ded.accountId, transferAccountId: null, date: form.payDate,
      }, liabilityIds.value))
    }
  }
  return results
})

// ─── Contribution preview ─────────────────────────────────────────────────────
const contributionPreview = computed(() => {
  const items = contributionItems(form.deductions, form.employerMatch)
  return items.map((item) => ({
    ...item,
    accountName: accountsStore.accounts.find((a) => a.id === item.accountId)?.name ?? `#${item.accountId}`,
  }))
})

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

async function save(mode: SaveMode = 'close') {
  // Editing always saves and closes; the "add another" modes only apply to new entries.
  if (props.editing) mode = 'close'
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
        updateBalance: updateBalance.value,
        createDepositTxn: createDepositTxn.value,
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
        updateBalance: updateBalance.value,
        createDepositTxn: createDepositTxn.value,
        createdAt: now,
      })
      toast.add({ title: 'Paycheck added', color: 'success' })
      // 'add' starts fresh; 'keep' leaves the entered values in place for the next entry.
      if (mode === 'add') resetForm()
    }
    // Close the modal only for plain saves; the "add another" modes keep it open.
    emit('saved', mode === 'close')
  } catch (err) {
    saveError.value = String(err)
    toast.add({ title: 'Failed to save paycheck', description: String(err), color: 'error' })
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <form class="space-y-5" @submit.prevent="save(saveMode)">

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

          <div v-if="duplicateMatch" class="rounded-lg border border-warning/30 bg-warning/10 p-3 space-y-2">
            <div class="flex items-start gap-2">
              <UIcon name="i-ph-warning" class="text-warning text-xl shrink-0 mt-0.5" />
              <div class="min-w-0">
                <p class="font-semibold text-sm">Possible duplicate deposit</p>
                <p class="text-xs text-muted">
                  An existing transaction already looks like this deposit: {{ money(duplicateMatch.amount) }} on
                  {{ DateTime.fromISO(duplicateMatch.date).toLocaleString(DateTime.DATE_MED) }}
                  ("{{ duplicateMatch.description }}"). No new deposit transaction will be created for this paycheck
                  unless you override below.
                </p>
              </div>
            </div>
            <UCheckbox v-model="createDepositTxn" label="Create a deposit transaction anyway" />
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

      <div class="w-full lg:w-1/2 bg-muted border border-default/50 rounded-xl flex flex-col space-y-4 lg:-my-4 lg:ml-3 p-4">
        <!-- Deductions -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <p class="text-xs font-semibold uppercase tracking-wide text-muted">Deductions</p>
            <UButton size="xs" variant="soft" icon="i-ph-plus" @click="addDeduction">Add</UButton>
          </div>
          <div v-for="(ded, i) in form.deductions" :key="i" class="rounded border border-default p-3">
            <div class="flex gap-2 items-center">
              <UInputMenu
                :model-value="ded.label"
                mode="autocomplete"
                :items="deductionComboItems"
                placeholder="Account or description"
                class="flex-1"
                @update:model-value="onDeductionLabelChange(ded, $event)"
              >
                <template #empty>
                  <span v-if="ded.label">Use "{{ ded.label }}"</span>
                  <span v-else>No accounts</span>
                </template>
              </UInputMenu>
              <CurrencyInput v-model="ded.amount" class="w-24" />
              <UCheckbox v-model="ded.preTax" label="Pre-tax" class="shrink-0" />
              <UButton size="xs" variant="ghost" color="error" icon="i-ph-x" @click="removeDeduction(i)" />
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
        <div v-if="contributionPreview.length > 0" class="rounded border border-default p-3 space-y-1 mt-auto">
          <p class="text-xs font-semibold uppercase tracking-wide text-muted">Contributions that will be created</p>
          <div v-for="item in contributionPreview" :key="`${item.accountId}:${item.label}`" class="flex justify-between text-sm">
            <span class="text-muted">{{ item.label }} → {{ item.accountName }}</span>
            <span class="tabular-nums text-success">{{ money(item.amount) }}</span>
          </div>
        </div>

        <!-- Update account balance -->
        <div v-if="hasLinkedAccounts" class="rounded-lg border border-default p-3 space-y-2">
          <USwitch v-model="updateBalance" label="Update account balance" />
          <p class="text-xs text-muted">
            Writes balance snapshots for linked accounts (deposit and contributions), so they show up in your net-worth history.
          </p>
          <div v-if="updateBalance" class="text-sm space-y-1">
            <div v-for="(line, li) in allBalancePreviews" :key="li" class="tabular-nums">
              {{ accountsStore.accounts.find((a) => a.id === line.accountId)?.name ?? `#${line.accountId}` }}:
              {{ money(line.from) }} → <strong>{{ money(line.to) }}</strong>
            </div>
          </div>
        </div>
      </div>
    </div>

    <p v-if="saveError" class="text-sm text-error">{{ saveError }}</p>

    <div class="flex justify-end gap-2 pt-2">
      <UButton v-if="props.editing" type="submit" :loading="saving" :disabled="saving">Save</UButton>
      <UFieldGroup v-else>
        <UButton type="submit" :loading="saving" :disabled="saving">{{ saveModeLabel }}</UButton>
        <UDropdownMenu :items="saveMenuItems" :content="{ align: 'end' }">
          <UButton
            type="button"
            color="primary"
            icon="i-ph-caret-down"
            aria-label="More save options"
            :disabled="saving"
          />
        </UDropdownMenu>
      </UFieldGroup>
    </div>

  </form>
</template>
