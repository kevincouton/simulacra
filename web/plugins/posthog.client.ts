import posthog from 'posthog-js'
import type { PostHog } from 'posthog-js'

export default defineNuxtPlugin(() => {
  const config = useRuntimeConfig()

  let client: PostHog | null = null

  const getClient = (): PostHog | null => {
    if (client) return client
    const key = config.public.posthogKey
    if (!key) return null
    client = posthog.init(key, {
      api_host: config.public.posthogHost || 'https://eu.posthog.com',
    }) as PostHog
    return client
  }

  getClient()

  return {
    provide: {
      posthog: getClient,
    },
  }
})
