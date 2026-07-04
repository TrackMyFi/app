import { ref } from 'vue'

const STORAGE_KEY = 'tmfi:attribute-month-end-paycheck'

// Module-level so Settings and Transactions share one reactive value; the
// preference is per-device (display-only — it never changes stored data).
const enabled = ref(localStorage.getItem(STORAGE_KEY) === '1')

/**
 * Preference: attribute a month-end paycheck (and its pre-tax contribution
 * rows) to the month it funds in cash-flow analytics. See
 * src/lib/transactions/attribution.ts for the mechanics.
 */
export function useMonthEndPaycheckAttribution() {
  function set(v: boolean) {
    enabled.value = v
    localStorage.setItem(STORAGE_KEY, v ? '1' : '0')
  }
  return { enabled, set }
}
