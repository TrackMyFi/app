import { onUnmounted, ref } from 'vue'

const prefersReduced = () =>
  typeof window !== 'undefined' &&
  !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches

// ease-out-quart: quick start, gentle settle — the figure "lands" into place.
const easeOutQuart = (t: number) => 1 - Math.pow(1 - t, 4)

/**
 * Tweens a single number between arbitrary values. Unlike `useReveal` (a 0→1
 * progress multiplier for synchronized reveals), this animates an absolute
 * value from wherever it is now to a new target — so a balance can roll up from
 * its old figure to the new one when it grows, making progress feel real.
 *
 * `set(v)` jumps instantly (first paint, or a quiet correction); `animateTo(v)`
 * rolls there. Honors `prefers-reduced-motion` by snapping to the target.
 *
 * Usage:
 *   const balance = useCountUp(0)
 *   balance.set(current)         // settle without motion
 *   balance.animateTo(next)      // roll up on a gain
 *   // template: {{ money(balance.value) }}
 */
export function useCountUp(initial = 0, durationMs = 700) {
  const value = ref(initial)
  let raf = 0

  function set(target: number) {
    cancelAnimationFrame(raf)
    value.value = target
  }

  function animateTo(target: number) {
    cancelAnimationFrame(raf)
    const from = value.value
    if (prefersReduced() || from === target) {
      value.value = target
      return
    }
    let start = 0
    const step = (ts: number) => {
      if (!start) start = ts
      const p = Math.min((ts - start) / durationMs, 1)
      value.value = from + (target - from) * easeOutQuart(p)
      if (p < 1) raf = requestAnimationFrame(step)
      else value.value = target
    }
    raf = requestAnimationFrame(step)
  }

  onUnmounted(() => cancelAnimationFrame(raf))

  return { value, set, animateTo }
}
