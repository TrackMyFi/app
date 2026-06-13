import { realMonthlyReturn } from './projection'

/**
 * Monthly contribution required so `presentValue` grows to `target` in exactly
 * `monthsToRetirement` months, using the monthly real return.
 *
 * Closed-form annuity payment: FV = PV·(1+r)^n + PMT·((1+r)^n − 1)/r
 *   → PMT = (target − PV·(1+r)^n) · r / ((1+r)^n − 1)
 *
 * Returns 0 when the present value alone already reaches the target.
 * Returns null when `monthsToRetirement <= 0` (no time left).
 */
export function requiredMonthlyContribution(
  presentValue: number, target: number,
  expectedReturnRate: number, inflationRate: number,
  monthsToRetirement: number,
): number | null {
  if (monthsToRetirement <= 0) return null
  const r = realMonthlyReturn(expectedReturnRate, inflationRate)
  const n = monthsToRetirement
  if (Math.abs(r) < 1e-9) {
    const pmt = (target - presentValue) / n
    return pmt <= 0 ? 0 : pmt
  }
  const growth = Math.pow(1 + r, n)
  const fvPresent = presentValue * growth
  if (fvPresent >= target) return 0
  const pmt = ((target - fvPresent) * r) / (growth - 1)
  return pmt <= 0 ? 0 : pmt
}
