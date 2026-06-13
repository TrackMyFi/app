import { ref } from 'vue'
import { defineStore } from 'pinia'
import { buildBudgetMonth } from '../lib/budget'
import type { BudgetMonthSummary, BudgetMonthTarget } from '../lib/budget'
import {
  listBudgetMonths,
  listBudgetTxns,
  getBudgetMonthTarget,
  setBudgetMonthTarget,
  getBudgetPaycheckSummary,
} from '../lib/api/budget'
import type { BudgetMonth } from '../lib/api/budget'
import { listPaychecks } from '../lib/api/paychecks'

export const useBudgetStore = defineStore('budget', () => {
  const months = ref<BudgetMonth[]>([])
  const selectedYear = ref<number>()
  const selectedMonth = ref<number>()
  const target = ref<BudgetMonthTarget | null>(null)
  const summary = ref<BudgetMonthSummary | null>(null)
  const activeSection = ref<'income' | 'savings' | 'fixed' | 'discretionary' | null>(null)
  const paycheckGrossMap = ref<Record<number, number>>({})

  async function loadMonths() {
    months.value = await listBudgetMonths()
  }

  async function load(year: number, month: number) {
    selectedYear.value = year
    selectedMonth.value = month

    const monthStr = String(month).padStart(2, '0')
    const lastDay = new Date(year, month, 0).getDate()
    const startDate = `${year}-${monthStr}-01`
    const endDate = `${year}-${monthStr}-${String(lastDay).padStart(2, '0')}`

    const [txns, rawTarget, paycheckSummary, paychecks] = await Promise.all([
      listBudgetTxns(year, month),
      getBudgetMonthTarget(year, month),
      getBudgetPaycheckSummary(year, month),
      listPaychecks({ startDate, endDate }),
    ])

    paycheckGrossMap.value = Object.fromEntries(paychecks.map((p) => [p.id, p.grossAmount]))

    summary.value = buildBudgetMonth(txns, paycheckSummary)

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
    loadMonths,
    load,
    setTarget,
  }
})
