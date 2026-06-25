import { computed, onMounted, ref, toValue, type MaybeRefOrGetter } from 'vue'

let uid = 0

// Resolve a semantic color to a concrete value Unovis can paint into SVG.
// Mirrors the pattern in TransactionChart: Tailwind/Nuxt UI semantic utilities
// aren't plain CSS vars, so we read the computed color off a throwaway element.
function resolveColor(className: string, fallback: string): string {
  const el = document.createElement('span')
  el.className = className
  el.style.cssText = 'position:absolute;width:0;height:0;visibility:hidden'
  document.body.appendChild(el)
  const c = getComputedStyle(el).color
  document.body.removeChild(el)
  return c || fallback
}

/**
 * Builds a vertical gradient that paints a line/area emerald above $0 and red
 * below it, switching crisply at the zero line. Pass the chart's y-values; the
 * gradient stop is positioned where zero falls within the data's range, so it
 * tracks the curve regardless of axis padding.
 *
 * Bind `defs` to <ZeroGradientDefs>, paint line + area with `paint`, and set the
 * area's `baseline` to `min` so its fill spans exactly the data range (keeping
 * the gradient aligned between the line and the area).
 */
export function useZeroThresholdGradient(values: MaybeRefOrGetter<number[]>) {
  const id = `tmfi-zero-grad-${uid++}`
  const positive = ref('#10b981')
  const negative = ref('#ef4444')

  onMounted(() => {
    positive.value = resolveColor('text-primary', positive.value)
    negative.value = resolveColor('text-error', negative.value)
  })

  const vals = computed(() => toValue(values))
  const min = computed(() => (vals.value.length ? Math.min(...vals.value) : 0))
  const max = computed(() => (vals.value.length ? Math.max(...vals.value) : 0))

  // Fraction from the top (max) of the range down to where $0 sits.
  const offset = computed(() => {
    const lo = min.value, hi = max.value
    if (lo >= 0) return 1 // never below zero → all emerald
    if (hi <= 0) return 0 // never above zero → all red
    return hi / (hi - lo)
  })

  const paint = `url(#${id})`
  const defs = computed(() => ({ id, offset: offset.value, positive: positive.value, negative: negative.value }))
  const pointColor = (v: number) => (v < 0 ? negative.value : positive.value)

  return { id, paint, min, offset, positive, negative, defs, pointColor }
}
