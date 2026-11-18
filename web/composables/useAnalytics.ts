type PostHogClient = ReturnType<typeof import('posthog-js').default.init>

export function useAnalytics() {
  const track = (event: string, properties?: Record<string, unknown>) => {
    try {
      ;(window as any).umami?.track(event, properties)
    } catch {
      /* silent fail */
    }
  }

  return { track }
}
