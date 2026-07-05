import { ref } from 'vue'
import { defineStore } from 'pinia'
import { DateTime } from 'luxon'
import { buildBudgetMonth } from '../lib/budget'
import type { BudgetMonthSummary, BudgetMonthTarget, PaycheckSummary } from '../lib/budget'
import {
  listBudgetMonths,
  listBudgetTxns,
  getBudgetMonthTarget,
  setBudgetMonthTarget,
} from '../lib/api/budget'
import type { BudgetMonth } from '../lib/api/budget'
import { listPaychecks } from '../lib/api/paychecks'
import { listAccounts } from '../lib/api/accounts'
import { useMonthEndPaycheckAttribution } from '../composables/useMonthEndPaycheckAttribution'
import { isMonthEndDate, isMonthEndPaycheckRow } from '../lib/transactions/attribution'
import type { Transaction } from '../lib/types/Transaction'
import type { Paycheck } from '../lib/types/Paycheck'

// 'yyyy-MM' the paycheck's cash flow counts toward when attribution is on:
// a month-end pay date funds the following month.
function fundedPeriod(payDate: string): string {
  const dt = DateTime.fromISO(payDate)
  return (isMonthEndDate(payDate) ? dt.plus({ months: 1 }) : dt).toFormat('yyyy-MM')
}

function summarizePaychecks(paychecks: Paycheck[]): PaycheckSummary {
  let grossIncome = 0
  let netIncome = 0
  let taxes = 0
  for (const p of paychecks) {
    grossIncome += p.grossAmount
    netIncome += p.netAmount
    taxes += p.federalTax + p.stateTax + p.localTax + p.socialSecurityTax + p.medicareTax
  }
  return { grossIncome, netIncome, taxes }
}

export const useBudgetStore = defineStore('budget', () => {
  const months = ref<BudgetMonth[]>([])
  const selectedYear = ref<number>()
  const selectedMonth = ref<number>()
  const target = ref<BudgetMonthTarget | null>(null)
  const summary = ref<BudgetMonthSummary | null>(null)
  const activeSection = ref<'income' | 'savings' | 'fixed' | 'discretionary' | null>(null)
  const paycheckGrossMap = ref<Record<number, number>>({})
  // Month-end paycheck rows pulled in from the previous month when the
  // attribution preference is on — kept with their real dates so the UI can
  // notate them as carried forward.
  const carriedIn = ref<Transaction[]>([])

  const { enabled: attributePaychecks } = useMonthEndPaycheckAttribution()

  async function loadMonths() {
    months.value = await listBudgetMonths()
  }

  async function load(year: number, month: number) {
    selectedYear.value = year
    selectedMonth.value = month

    const monthDt = DateTime.local(year, month, 1)
    const period = monthDt.toFormat('yyyy-MM')
    const endDate = monthDt.endOf('month').toISODate()!
    // With attribution on, the previous month's month-end paycheck (and its
    // pre-tax contribution rows) counts toward this month, and this month's
    // own month-end paycheck shifts out — mirroring the Transactions page.
    const shift = attributePaychecks.value
    const prevDt = monthDt.minus({ months: 1 })
    const startDate = (shift ? prevDt : monthDt).toISODate()!

    const [txns, prevTxns, rawTarget, paychecks, accounts] = await Promise.all([
      listBudgetTxns(year, month),
      shift ? listBudgetTxns(prevDt.year, prevDt.month) : Promise.resolve([] as Transaction[]),
      getBudgetMonthTarget(year, month),
      listPaychecks({ startDate, endDate }),
      listAccounts(),
    ])

    paycheckGrossMap.value = Object.fromEntries(paychecks.map((p) => [p.id, p.grossAmount]))

    // Only paycheck-generated rows shift months; everything else always counts
    // toward its own calendar month.
    const effectiveTxns = shift
      ? [...prevTxns, ...txns].filter((t) => {
          const funded = isMonthEndPaycheckRow(t)
            ? DateTime.fromISO(t.date).plus({ months: 1 }).toFormat('yyyy-MM')
            : t.date.slice(0, 7)
          return funded === period
        })
      : txns
    carriedIn.value = shift ? effectiveTxns.filter((t) => !t.date.startsWith(period)) : []

    const paycheckSummary = summarizePaychecks(
      paychecks.filter((p) => (shift ? fundedPeriod(p.payDate) : p.payDate.slice(0, 7)) === period),
    )

    summary.value = buildBudgetMonth(effectiveTxns, paycheckSummary, accounts)

    if (rawTarget) {
      target.value = {
        savingsTarget: rawTarget.savingsTarget,
        sourceYear: rawTarget.sourceYear,
        sourceMonth: rawTarget.sourceMonth,
        isInherited: rawTarget.sourceYear !== year || rawTarget.sourceMonth !== month,
      }
    } else {
      target.value = null
    }

    if (!activeSection.value) {
      activeSection.value = 'income'
    }
  }

  async function setTarget(savingsTarget: number) {
    if (selectedYear.value == null || selectedMonth.value == null) return
    await setBudgetMonthTarget(selectedYear.value, selectedMonth.value, savingsTarget)
    const rawTarget = await getBudgetMonthTarget(selectedYear.value, selectedMonth.value)
    if (rawTarget) {
      target.value = {
        savingsTarget: rawTarget.savingsTarget,
        sourceYear: rawTarget.sourceYear,
        sourceMonth: rawTarget.sourceMonth,
        isInherited:
          rawTarget.sourceYear !== selectedYear.value || rawTarget.sourceMonth !== selectedMonth.value,
      }
    }
  }

  return {
    months,
    selectedYear,
    selectedMonth,
    target,
    summary,
    activeSection,
    paycheckGrossMap,
    carriedIn,
    loadMonths,
    load,
    setTarget,
  }
})
