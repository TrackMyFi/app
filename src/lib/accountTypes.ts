export const ACCOUNT_TYPES = [
  'checking', 'savings', 'brokerage', '401k', 'roth_401k',
  'traditional_ira', 'roth_ira', 'hsa', 'real_estate', 'crypto', 'liability',
] as const
export type AccountType = typeof ACCOUNT_TYPES[number]

export const ACCOUNT_TYPE_LABELS: Record<AccountType, string> = {
  checking: 'Checking',
  savings: 'Savings',
  brokerage: 'Brokerage',
  '401k': '401(k)',
  roth_401k: 'Roth 401(k)',
  traditional_ira: 'Traditional IRA',
  roth_ira: 'Roth IRA',
  hsa: 'HSA',
  real_estate: 'Real Estate',
  crypto: 'Crypto',
  liability: 'Liability',
}

export const labelForAccountType = (type: string): string =>
  ACCOUNT_TYPE_LABELS[type as AccountType] ?? type

export const accountTypeItems = ACCOUNT_TYPES.map((t) => ({
  label: ACCOUNT_TYPE_LABELS[t],
  value: t,
}))

export const INVESTMENT_TYPES = new Set<string>(
  ['brokerage','401k','roth_401k','traditional_ira','roth_ira','hsa','crypto'],
)
export const isInvestment = (t: string) => INVESTMENT_TYPES.has(t)
export const isLiability = (t: string) => t === 'liability'
export const defaultIncludeInFire = (t: AccountType) => INVESTMENT_TYPES.has(t)

export const investmentTypeItems = [...INVESTMENT_TYPES].map((t) => ({
  label: ACCOUNT_TYPE_LABELS[t as AccountType] ?? t,
  value: t,
}))
