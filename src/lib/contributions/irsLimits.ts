export interface YearLimits {
  k401: number // 401k + roth_401k combined employee elective deferral
  ira: number // traditional_ira + roth_ira combined
  hsaSelf: number
  hsaFamily: number
  k401CatchUpAge: number
  k401CatchUp: number
  iraCatchUpAge: number
  iraCatchUp: number
  hsaCatchUpAge: number
  hsaCatchUp: number
}

// Update this table each year when the IRS announces new limits (typically Oct/Nov).
const IRS_LIMITS: Record<number, YearLimits> = {
  2024: {
    k401: 23000, ira: 7000, hsaSelf: 4150, hsaFamily: 8300,
    k401CatchUpAge: 50, k401CatchUp: 7500,
    iraCatchUpAge: 50, iraCatchUp: 1000,
    hsaCatchUpAge: 55, hsaCatchUp: 1000,
  },
  2025: {
    k401: 23500, ira: 7000, hsaSelf: 4300, hsaFamily: 8550,
    k401CatchUpAge: 50, k401CatchUp: 7500,
    iraCatchUpAge: 50, iraCatchUp: 1000,
    hsaCatchUpAge: 55, hsaCatchUp: 1000,
  },
  2026: {
    k401: 23500, ira: 7000, hsaSelf: 4400, hsaFamily: 8700,
    k401CatchUpAge: 50, k401CatchUp: 7500,
    iraCatchUpAge: 50, iraCatchUp: 1000,
    hsaCatchUpAge: 55, hsaCatchUp: 1000,
  },
}

export interface ResolvedLimits {
  limits: YearLimits
  estimated: boolean
  estimatedFrom?: number
}

/**
 * Resolve IRS limits for a year. If the exact year is known, returns it
 * un-estimated. Otherwise clamps to the known range — years above the max use
 * the latest known year; any other unknown year (below the min, or an in-range
 * gap) uses the earliest known year — and flags `estimated` with the source
 * year in `estimatedFrom`. The table is maintained contiguously, so in-range
 * gaps do not occur in practice.
 */
export function resolveYearLimits(year: number): ResolvedLimits {
  if (IRS_LIMITS[year]) {
    return { limits: IRS_LIMITS[year], estimated: false }
  }
  const knownYears = Object.keys(IRS_LIMITS).map(Number).sort((a, b) => a - b)
  const min = knownYears[0]
  const max = knownYears[knownYears.length - 1]
  const source = year > max ? max : min
  return { limits: IRS_LIMITS[source], estimated: true, estimatedFrom: source }
}
