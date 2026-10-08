export default defineNuxtConfig({
  ssr: false,
  nitro: {
    prerender: {
      routes: ['/'],
    },
  },
  app: {
    head: {
      title: 'Simulacra — Synthetic Human Simulation Playground',
      meta: [
        { charset: 'utf-8' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      ],
    },
  },
})
