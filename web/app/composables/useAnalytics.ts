
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
