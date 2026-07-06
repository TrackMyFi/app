export interface FireAccount {
  id: number
  type: string
  includeInFireCalculations: boolean
  /** Traditional (pre-tax) share of a mixed 401k, 0..1; null/absent when unset or not applicable. */
  traditionalPct?: number | null
}
export interface FireBalance { accountId: number; balance: number; recordedAt: string; id?: number } // ISO date
