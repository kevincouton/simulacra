import posthog from 'posthog-js'
import type { PostHog } from 'posthog-js'

export default defineNuxtPlugin(() => {
  const config = useRuntimeConfig()

  const key = config.public.posthogKey
  let client: PostHog | null = null

  if (key) {
    client = posthog.init(key, {
      api_host: config.public.posthogHost || 'https://eu.posthog.com',
    }) as PostHog
  }

  const getClient = (): PostHog | null => client

  return {
    provide: {
      posthog: getClient,
    },
  }
})
