import { invoke } from '@tauri-apps/api/core'
import type { Paycheck } from '../types/Paycheck'

export interface PaycheckFilter {
  startDate?: string | null
  endDate?: string | null
  employer?: string | null
}

export interface PaycheckDeductionInput {
  label: string
  amount: number
  preTax: boolean
  contributionAccountType?: string | null
  accountId?: number | null
}

export interface EmployerMatchInput {
  label: string
  amount: number
  accountId?: number | null
}

export interface NewPaycheck {
  payDate: string
  employer: string
  payPeriod: string
  grossAmount: number
  netAmount: number
  federalTax: number
  stateTax: number
  localTax: number
  socialSecurityTax: number
  medicareTax: number
  deductions: PaycheckDeductionInput[]
  employerMatch: EmployerMatchInput[]
  createdAt: string
}

export interface UpdatePaycheck {
  id: number
  payDate: string
  employer: string
  payPeriod: string
  grossAmount: number
  netAmount: number
  federalTax: number
  stateTax: number
  localTax: number
  socialSecurityTax: number
  medicareTax: number
  deductions: PaycheckDeductionInput[]
  employerMatch: EmployerMatchInput[]
  updatedAt: string
}

export const listPaychecks = (filter: PaycheckFilter = {}) =>
  invoke<Paycheck[]>('list_paychecks_cmd', { filter })
export const getPaycheck = (id: number) =>
  invoke<Paycheck>('get_paycheck_cmd', { id })
export const createPaycheck = (paycheck: NewPaycheck) =>
  invoke<Paycheck>('create_paycheck_cmd', { paycheck })
export const updatePaycheck = (paycheck: UpdatePaycheck) =>
  invoke<Paycheck>('update_paycheck_cmd', { paycheck })
export const deletePaycheck = (id: number) =>
  invoke<void>('delete_paycheck_cmd', { id })
