import { test, expect } from '@playwright/test'

// Persona 5 — search engines and LLM crawlers reading the tracker.

test('the index carries full SEO/AEO metadata', async ({ page }) => {
  await page.goto('/')
  await expect(page).toHaveTitle(/Synthetic Simulation Stack/)

  const description = await page.locator('meta[name="description"]').getAttribute('content')
  expect(description?.length).toBeGreaterThan(50)

  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', 'https://simulation.lucanian.app/')
  await expect(page.locator('meta[property="og:title"]')).toHaveAttribute('content', /Simulacra/)

  // JSON-LD graph: WebSite + FAQPage + ItemList on the index.
  const ld = await page.locator('script[type="application/ld+json"]').innerText()
  const graph = JSON.parse(ld)
  const types = graph.map((node: any) => node['@type'])
  expect(types).toContain('WebSite')
  expect(types).toContain('FAQPage')
  expect(types).toContain('ItemList')

  const faq = graph.find((n: any) => n['@type'] === 'FAQPage')
  const questions = faq.mainEntity.map((q: any) => q.name)
  expect(questions).toContain('What is synthetic human simulation?')
  expect(questions).toContain('Does Simulacra expose machine-readable endpoints?')
})

test('stage pages carry TechArticle structured data and canonical URLs', async ({ page }) => {
  await page.goto('/01-reward.html')
  await expect(page.locator('h1')).toHaveText(/Reward Signal/)
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    'href',
    'https://simulation.lucanian.app/01-reward.html',
  )
  const ld = JSON.parse(await page.locator('script[type="application/ld+json"]').innerText())
  expect(ld.some((n: any) => n['@type'] === 'TechArticle')).toBe(true)

  // The Agents & API page is part of the tracked site.
  await page.goto('/agents.html')
  await expect(page.locator('h1')).toHaveText(/Agents & API/)
  await expect(page.locator('main')).toContainText('/mcp')
  await expect(page.locator('main')).toContainText('agent.json')
})

test('discovery files: robots, sitemap, llms.txt', async ({ request }) => {
  const robots = await (await request.get('/robots.txt')).text()
  expect(robots).toContain('Allow: /')
  expect(robots).toContain('Sitemap: https://simulation.lucanian.app/sitemap.xml')

  const sitemap = await (await request.get('/sitemap.xml')).text()
  for (const page of ['', '01-reward.html', '07-subject.html', '10-market.html', 'agents.html']) {
    expect(sitemap, `sitemap missing ${page}`).toContain(`https://simulation.lucanian.app/${page}<`)
  }

  const llms = await (await request.get('/llms.txt')).text()
  expect(llms).toContain('# Simulacra')
  expect(llms).toContain('POST https://simulation.lucanian.app/mcp')
  expect(llms).toContain('/.well-known/agent.json')
  expect(llms).toContain('run_town')
})

test('agent discovery: the A2A card is served with a JSON content type', async ({ request }) => {
  const res = await request.get('/.well-known/agent.json')
  expect(res.status()).toBe(200)
  expect(res.headers()['content-type']).toContain('application/json')
})
