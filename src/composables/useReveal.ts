import { onUnmounted, ref } from 'vue'

const prefersReduced = () =>
  typeof window !== 'undefined' &&
  !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches

// ease-out-quart: fast start, gentle settle — numbers feel like they "land".
const easeOutQuart = (t: number) => 1 - Math.pow(1 - t, 4)

/**
 * Drives a 0→1 progress value that eases in when `play()` is called. Multiply
 * displayed numbers and bar widths by it for a synchronized count-up reveal,
 * so the page's figures "tick up" into place when data lands or the year
 * changes — making progress feel real rather than just appearing.
 *
 * Honors `prefers-reduced-motion` by jumping straight to 1 (no animation).
 *
 * Usage:
 *   const { progress: reveal, play } = useReveal()
 *   // ...after data loads: play()
 *   // template: {{ money(total * reveal) }}
 */
export function useReveal(durationMs = 750) {
  const progress = ref(prefersReduced() ? 1 : 0)
  let raf = 0

  function play() {
    cancelAnimationFrame(raf)
    if (prefersReduced()) {
      progress.value = 1
      return
    }
    progress.value = 0
    let start = 0
    const step = (ts: number) => {
      if (!start) start = ts
      const p = Math.min((ts - start) / durationMs, 1)
      progress.value = easeOutQuart(p)
      if (p < 1) raf = requestAnimationFrame(step)
      else progress.value = 1
    }
    raf = requestAnimationFrame(step)
  }

  onUnmounted(() => cancelAnimationFrame(raf))

  return { progress, play }
}
