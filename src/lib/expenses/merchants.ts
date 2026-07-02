import { classifyFlow, type FlowBucket } from '../transactions/flow'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

type AccountLookup = Map<number, Account> | Account[]

/**
 * Transactions carry only a free-text `description` — no merchant field. These
 * are the common import-format prefixes and trailing noise (store numbers,
 * dates, processor reference codes) stripped so repeat payees cluster together.
 * Best-effort by design: a personal local ledger doesn't need perfect merchant
 * identity, just a useful approximation.
 */
const NOISE_PREFIXES = [
  /^POS(\s+DEBIT)?\s+/i,
  /^ACH\s+(DEBIT|CREDIT)\s+/i,
  /^CHECKCARD\s+/i,
  /^DEBIT\s+CARD\s+PURCHASE\s+/i,
  /^PURCHASE\s+(AUTHORIZED\s+)?/i,
  /^RECURRING\s+PAYMENT\s+/i,
  /^PAYPAL\s*\*/i,
  /^SQ\s*\*/i,
  /^TST\*\s*/i,
]

// Trailing store numbers ("#4471"), dates ("06/12" or "06/12/26"), and long
// reference codes that contain at least one digit — stripped repeatedly from
// the end of the string. Requiring a digit keeps this from eating plain
// trailing words ("...COFFEE", "...PURCHASE") that just happen to be long.
const TRAILING_NOISE = /[\s#*-]+(\d{1,2}\/\d{1,2}(\/\d{2,4})?|(?=[A-Z0-9]*\d)[A-Z0-9]{5,}|\d{2,})$/i

function stripTrailingNoise(s: string): string {
  let out = s
  let prev: string
  do {
    prev = out
    out = out.replace(TRAILING_NOISE, '').trim()
  } while (out !== prev && out.length > 0)
  return out
}

/** The description with import noise stripped — a best-effort substring of the original text. */
export function coreText(description: string): string {
  let s = description.trim().replace(/\s+/g, ' ')
  for (const re of NOISE_PREFIXES) s = s.replace(re, '')
  // A prefix strip (e.g. "CHECKCARD ") often leaves a leading MMDD posting date behind.
  s = s.replace(/^\d{4}\s+/, '')
  s = stripTrailingNoise(s)
  // Trailing web TLD reads as noise once clustered ("AMAZON.COM" → "Amazon").
  s = s.replace(/\.(com|net|org|co|io)$/i, '')
  return s.trim()
}

/** Case-folded grouping key for clustering repeat payees. */
export function normalizeKey(description: string): string {
  return coreText(description).toUpperCase()
}

// Descriptions that strip down to near-nothing informative — too generic to
// cluster meaningfully, so they're pooled into a single catch-all bucket
// instead of either scattering as noisy singleton "merchants" or wrongly
// merging unrelated purchases that just share a generic label.
const GENERIC_KEYS = new Set([
  'DEBIT', 'CREDIT', 'PAYMENT', 'PURCHASE', 'TRANSFER', 'WITHDRAWAL', 'DEPOSIT',
  'CHECK', 'ATM', 'ATM WITHDRAWAL', 'ONLINE PAYMENT', 'ONLINE TRANSFER', 'WIRE TRANSFER',
  'ACH TRANSFER', 'MOBILE DEPOSIT', 'DEBIT CARD PURCHASE', 'MISC', 'MISCELLANEOUS', 'OTHER',
])

export function isGenericKey(key: string): boolean {
  if (key.length < 3) return true
  if (/^[\d\s#*-]+$/.test(key)) return true
  return GENERIC_KEYS.has(key)
}

/** Title-cased, human-readable name for a normalized description. */
export function displayName(description: string): string {
  const core = coreText(description) || description.trim()
  return core
    .toLowerCase()
    .split(' ')
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ')
}

export interface MerchantGroup {
  key: string
  displayName: string
  /** Literal substring of the original descriptions — safe to use as a Transactions search filter. */
  searchTerm: string
  /** Dominant category bucket among this merchant's transactions. */
  category: FlowBucket
  total: number
  count: number
  /** Share of total grouped spend (0–1). */
  share: number
}

export const OTHER_MERCHANT_KEY = '__OTHER__'

/**
 * Groups real spending (expenses; excludes savings/contributions and transfers,
 * which carry no merchant identity) by best-effort merchant name.
 */
export function groupByMerchant(transactions: Transaction[], accounts: AccountLookup): MerchantGroup[] {
  interface Bucket { total: number; count: number; searchTerm: string; byCategory: Map<FlowBucket, number> }
  const groups = new Map<string, Bucket>()
  let grandTotal = 0

  for (const t of transactions) {
    const flow = classifyFlow(t, accounts)
    if (flow.isSavings || flow.outflow <= 0 || !flow.bucket) continue

    const key = normalizeKey(t.description) || OTHER_MERCHANT_KEY
    const effectiveKey = isGenericKey(key) ? OTHER_MERCHANT_KEY : key

    let g = groups.get(effectiveKey)
    if (!g) {
      g = { total: 0, count: 0, searchTerm: coreText(t.description), byCategory: new Map() }
      groups.set(effectiveKey, g)
    }
    g.total += flow.outflow
    g.count += 1
    g.byCategory.set(flow.bucket, (g.byCategory.get(flow.bucket) ?? 0) + flow.outflow)
    grandTotal += flow.outflow
  }

  const result: MerchantGroup[] = []
  for (const [key, g] of groups) {
    const dominant = [...g.byCategory.entries()].sort((a, b) => b[1] - a[1])[0][0]
    result.push({
      key,
      displayName: key === OTHER_MERCHANT_KEY ? 'Other purchases' : displayName(g.searchTerm),
      searchTerm: key === OTHER_MERCHANT_KEY ? '' : g.searchTerm,
      category: dominant,
      total: g.total,
      count: g.count,
      share: grandTotal > 0 ? g.total / grandTotal : 0,
    })
  }

  return result.sort((a, b) => b.total - a.total)
}
