export default defineNuxtConfig({
  ssr: false,
  nitro: {
    prerender: {
      routes: ['/'],
    },
  },
  // Served under /app/ by simulacra-server (the tracker owns the root for SEO).
  app: {
    baseURL: '/app/',
    head: {
      title: 'Simulacra — Synthetic Human Simulation Playground',
      meta: [
        { charset: 'utf-8' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        { name: 'robots', content: 'noindex' },
      ],
    },
  },
})
