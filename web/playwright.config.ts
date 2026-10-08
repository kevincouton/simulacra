import { defineConfig } from '@playwright/test'

// End-to-end suites for the whole simulacra deployment (tracker + UI +
// API + MCP + A2A). Defaults to the live site; point
// PLAYWRIGHT_BASE_URL at a local simulacra-server for pre-deploy checks.
export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.spec.ts',
  fullyParallel: true,
  retries: process.env.CI ? 1 : 0,
  reporter: [['list']],
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? 'https://simulation.lucanian.app',
  },
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }],
})
