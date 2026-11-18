type PostHogClient = ReturnType<typeof import('posthog-js').default.init>

export function useAnalytics() {
  const { $posthog } = useNuxtApp()

  const track = (event: string, properties?: Record<string, unknown>) => {
    $posthog?.()?.capture(event, properties)
    try {
      ;(window as any).umami?.track(event, properties)
    } catch {
      /* silent fail */
    }
  }

  return { track }
}
