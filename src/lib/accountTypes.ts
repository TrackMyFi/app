export const ACCOUNT_TYPES = [
  'checking', 'savings', 'brokerage', '401k', 'roth_401k',
  'traditional_ira', 'roth_ira', 'hsa', 'real_estate', 'crypto', 'liability',
] as const
export type AccountType = typeof ACCOUNT_TYPES[number]

export const INVESTMENT_TYPES = new Set<string>(
  ['brokerage','401k','roth_401k','traditional_ira','roth_ira','hsa','crypto'],
)
export const isInvestment = (t: string) => INVESTMENT_TYPES.has(t)
export const isLiability = (t: string) => t === 'liability'
export const defaultIncludeInFire = (t: AccountType) => INVESTMENT_TYPES.has(t)
