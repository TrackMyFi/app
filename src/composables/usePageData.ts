import { ref } from 'vue'

/**
 * Wraps a page's async data load with loading + error state so a failed initial
 * fetch never leaves a page silently blank.
 *
 * Without this, an unhandled rejection in a component's `onMounted` (e.g. a read
 * that hits a momentarily-locked DB during the startup sync catch-up) swallows the
 * error and the page renders nothing. `run` captures the loader so `retry()` can
 * re-invoke it after a transient failure.
 *
 * Usage:
 *   const { loading, error, run, retry } = usePageData()
 *   onMounted(() => run(async () => { ...load stores... }))
 *   // template: <PageError v-if="error" :message="error" @retry="retry" />
 */
export function usePageData() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  let lastLoader: (() => Promise<void>) | null = null

  async function run(loader?: () => Promise<void>): Promise<void> {
    if (loader) lastLoader = loader
    if (!lastLoader) return
    loading.value = true
    error.value = null
    try {
      await lastLoader()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      loading.value = false
    }
  }

  const retry = () => run()

  return { loading, error, run, retry }
}
