import { classifyFlow } from '../transactions/flow'
import { coreText, normalizeKey, isGenericKey, resolveVendor, titleCaseWords, type VendorRuleInput } from './merchants'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

type AccountLookup = Map<number, Account> | Account[]

export interface VendorRuleSuggestion {
  key: string
  keyword: string
  vendorName: string
  count: number
  total: number
  /** A few of the distinct raw descriptions this suggestion would merge. */
  sampleDescriptions: string[]
}

const MIN_GROUPS_TO_SUGGEST = 2
const MAX_SUGGESTIONS = 10
const MAX_SAMPLES = 3

function words(s: string): string[] {
  return s.split(' ').filter(Boolean)
}

/** Too short or containing a digit — a store number/reference code, not part of a vendor's name. */
function isNoiseWord(w: string): boolean {
  return w.length < 3 || /\d/.test(w)
}

function wordwiseLcp(wordLists: string[][]): string[] {
  const first = wordLists[0]
  const lcp: string[] = []
  for (let i = 0; i < first.length; i++) {
    const w = first[i].toUpperCase()
    if (!wordLists.every((ws) => (ws[i] ?? '').toUpperCase() === w)) break
    lcp.push(first[i])
  }
  return lcp
}

/**
 * Truncates at the first noisy word rather than trimming only from the end —
 * a shared prefix like "Pizza Hut 029908 Hebron" has its noise in the middle
 * (the store number), with a clean-looking city name still trailing it.
 */
function cleanLcp(lcp: string[]): string[] {
  const firstNoiseIdx = lcp.findIndex(isNoiseWord)
  return firstNoiseIdx === -1 ? lcp : lcp.slice(0, firstNoiseIdx)
}

/**
 * Surfaces vendors whose spending is currently splintered across several
 * distinct merchant groups — e.g. "Amazon Mktpl*0U36P84J3" and "Amazon
 * Mktpl*T45SK5WG3" read as different merchants because the order-reference
 * suffix defeats the regex-based normalizer in merchants.ts. Groups that share
 * a first word are clustered, and the suggested rule is the longest common
 * word-prefix across the cluster, trimmed of trailing noisy tokens (store
 * numbers, reference codes) — a starting guess the user can tweak before
 * saving. Transactions already covered by an existing rule are excluded, so
 * accepting a suggestion makes it disappear next time.
 */
export function suggestVendorRules(
  transactions: Transaction[],
  accounts: AccountLookup,
  existingRules: VendorRuleInput[],
): VendorRuleSuggestion[] {
  interface Group { searchTerm: string; count: number; total: number }
  const groups = new Map<string, Group>()

  for (const t of transactions) {
    const flow = classifyFlow(t, accounts)
    if (flow.isSavings || flow.outflow <= 0 || !flow.bucket) continue

    const resolved = resolveVendor(t.description, existingRules)
    if (resolved.key.startsWith('VENDOR::')) continue // already covered by a rule

    const key = normalizeKey(t.description)
    if (!key || isGenericKey(key)) continue

    let g = groups.get(key)
    if (!g) {
      g = { searchTerm: coreText(t.description), count: 0, total: 0 }
      groups.set(key, g)
    }
    g.count += 1
    g.total += flow.outflow
  }

  const byFirstWord = new Map<string, Group[]>()
  for (const g of groups.values()) {
    const first = words(g.searchTerm)[0]
    if (!first || isNoiseWord(first)) continue
    const bucket = byFirstWord.get(first.toUpperCase()) ?? []
    bucket.push(g)
    byFirstWord.set(first.toUpperCase(), bucket)
  }

  const suggestions: VendorRuleSuggestion[] = []
  for (const members of byFirstWord.values()) {
    if (members.length < MIN_GROUPS_TO_SUGGEST) continue

    const lcp = cleanLcp(wordwiseLcp(members.map((m) => words(m.searchTerm))))

    const keyword = lcp.join(' ').toLowerCase()
    const vendorName = titleCaseWords(lcp.join(' '))
    const count = members.reduce((s, m) => s + m.count, 0)
    const total = members.reduce((s, m) => s + m.total, 0)
    const sampleDescriptions = [...members]
      .sort((a, b) => b.total - a.total)
      .slice(0, MAX_SAMPLES)
      .map((m) => m.searchTerm)

    suggestions.push({ key: `SUGGEST::${keyword.toUpperCase()}`, keyword, vendorName, count, total, sampleDescriptions })
  }

  return suggestions.sort((a, b) => b.total - a.total).slice(0, MAX_SUGGESTIONS)
}
