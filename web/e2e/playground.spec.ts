import { test, expect } from '@playwright/test'

// Persona 1 — the researcher driving the playground UI in a browser.
// See flows.md. UI output is cross-checked against the JSON API so the
// tests stay correct whatever the seeded values happen to be.

const SECTIONS = ['town', 'variance', 'coevolution', 'gossip'] as const

async function apiJson(request: import('@playwright/test').APIRequestContext, path: string) {
  const res = await request.get(path)
  expect(res.ok(), `GET ${path} failed: ${res.status()}`).toBeTruthy()
  return res.json()
}

test.describe('landing', () => {
  test('shows title, h1, and all four experiment sections', async ({ page }) => {
    await page.goto('/app/')
    await expect(page).toHaveTitle(/Simulacra/)
    await expect(page.locator('h1')).toHaveText('Simulacra')
    for (const id of SECTIONS) {
      await expect(page.locator(`section#${id}`)).toBeVisible()
      await expect(page.locator(`section#${id} h2`)).toBeVisible()
    }
    await expect(page.locator('section#town .placeholder')).toHaveText(/Enter a seed/)
  })

  test('nav anchors jump to sections', async ({ page }) => {
    await page.goto('/app/')
    await page.locator('.nav-links a[href="#gossip"]').click()
    await expect(page).toHaveURL(/#gossip$/)
    await expect(page.locator('section#gossip')).toBeInViewport()
  })
})

test.describe('experiment runs', () => {
  test('town run renders a per-agent table consistent with the API', async ({ page, request }) => {
    const api = await apiJson(request, '/api/town?seed=42')
    const section = page.locator('section#town')
    await page.goto('/app/')
    await section.locator('input[type=number]').fill('42')
    await section.getByRole('button', { name: 'Run', exact: true }).click()

    const summary = section.locator('.summary')
    await expect(summary).toBeVisible()
    await expect(summary).toContainText(`${api.results.length} attempts`)
    await expect(summary).toContainText(`${(api.success_rate * 100).toFixed(1)}%`)

    const rows = section.locator('tbody tr:not(.violation-row)')
    await expect(rows).toHaveCount(api.results.length)

    // Badge consistency: every FAIL expands a violation row, PASS never does.
    const badges = section.locator('tbody .badge')
    await expect(badges).toHaveCount(api.results.length)
    const failCount = api.results.filter((r: any) => !(r.reward >= 1 && r.violations.length === 0)).length
    await expect(section.locator('tbody tr.violation-row')).toHaveCount(failCount)
    for (const badge of await badges.all()) {
      await expect(badge).toContainText(/PASS|FAIL/)
    }
  })

  test('variance run cross-checks its verdict banner against the API', async ({ page, request }) => {
    const api = await apiJson(request, '/api/variance?seed=42')
    const section = page.locator('section#variance')
    await page.goto('/app/')
    await section.locator('input[type=number]').fill('42')
    await section.getByRole('button', { name: 'Run', exact: true }).click()

    await expect(section.locator('.hbar-row')).toHaveCount(api.per_run_rates.length)
    await expect(section.locator('.verdict')).toContainText(`empirical ${api.empirical_mean.toFixed(3)}`)

    const expectMatch = api.mean_match && api.variance_match
    const banner = section.locator('.banner')
    await expect(banner).toHaveText(new RegExp(`DISTRIBUTION ${expectMatch ? 'MATCH' : 'MISMATCH'}`))
    await expect(banner).toHaveClass(expectMatch ? /pass/ : /fail/)
  })

  test('co-evolution run renders every round', async ({ page, request }) => {
    const api = await apiJson(request, '/api/coevolution?seed=42')
    const section = page.locator('section#coevolution')
    await page.goto('/app/')
    await section.locator('input[type=number]').fill('42')
    await section.getByRole('button', { name: 'Run', exact: true }).click()

    const rows = section.locator('.rounds-table tbody tr')
    await expect(rows).toHaveCount(api.rounds.length)
    await expect(rows.first().locator('td').first()).toHaveText('0')
    await expect(rows.nth(api.rounds.length - 1).locator('td').first()).toHaveText(String(api.rounds.length - 1))
    await expect(section.locator('.rounds-table .hbar-fill').first()).toBeVisible()
  })

  test('gossip run shows both rates and the delta conclusion', async ({ page, request }) => {
    const api = await apiJson(request, '/api/gossip?seed=42')
    const section = page.locator('section#gossip')
    await page.goto('/app/')
    await section.locator('input[type=number]').fill('42')
    await section.getByRole('button', { name: 'Run', exact: true }).click()

    const cards = section.locator('.card-value')
    await expect(cards).toHaveCount(2)
    await expect(cards.nth(0)).toHaveText(`${(api.no_gossip_rate * 100).toFixed(1)}%`)
    await expect(cards.nth(1)).toHaveText(`${(api.gossip_rate * 100).toFixed(1)}%`)
    await expect(section.locator('.conclusion')).toContainText(
      `${((api.gossip_rate - api.no_gossip_rate) * 100).toFixed(1)} points`,
    )
  })

  test('runs are deterministic: same seed twice gives identical output', async ({ page }) => {
    const section = page.locator('section#town')
    await page.goto('/app/')
    await section.locator('input[type=number]').fill('42')
    await section.getByRole('button', { name: 'Run', exact: true }).click()
    const first = await section.locator('.summary').innerText()

    await section.getByRole('button', { name: 'Run', exact: true }).click()
    await expect(section.locator('.summary')).toBeVisible()
    const second = await section.locator('.summary').innerText()
    expect(second).toEqual(first)
  })
})

test.describe('shareable deep links', () => {
  test('?section=gossip&seed=7&run=1 auto-runs and matches the API', async ({ page, request }) => {
    const api = await apiJson(request, '/api/gossip?seed=7')
    await page.goto('/app/?section=gossip&seed=7&run=1')
    const section = page.locator('section#gossip')
    const cards = section.locator('.card-value')
    await expect(cards.nth(1)).toHaveText(`${(api.gossip_rate * 100).toFixed(1)}%`)
    await expect(section.locator('input[type=number]')).toHaveValue('7')
    await expect(page).toHaveURL(/section=gossip/)
  })

  test('clicking Run records the run in the URL', async ({ page }) => {
    const section = page.locator('section#variance')
    await page.goto('/app/')
    await section.locator('input[type=number]').fill('11')
    await section.getByRole('button', { name: 'Run', exact: true }).click()
    await expect(section.locator('.hbar-row').first()).toBeVisible()
    await expect(page).toHaveURL(/section=variance/)
    await expect(page).toHaveURL(/seed=11/)
  })
})

test.describe('failure states', () => {
  test('API outage shows the recovery placeholder, not stale data', async ({ page }) => {
    await page.route('**/api/town*', (route) => route.abort())
    const section = page.locator('section#town')
    await page.goto('/app/')
    await section.getByRole('button', { name: 'Run', exact: true }).click()
    await expect(section.locator('.placeholder')).toHaveText(/Start the simulacra server/)
    await expect(section.locator('.summary')).toHaveCount(0)
  })
})
