import { test, expect } from '@playwright/test'

// Persona 2 — the integrator embedding the raw JSON API.

const ENDPOINTS: Array<{ path: string; fields: string[]; extra?: (d: any) => void }> = [
  {
    path: '/api/town?seed=42',
    fields: ['seed', 'results', 'success_rate', 'total_reward'],
    extra: (d) => expect(d.results).toHaveLength(20), // 5 worlds x 4 personas
  },
  {
    path: '/api/variance?seed=42',
    fields: ['seed', 'per_run_rates', 'empirical_mean', 'theoretical_mean', 'mean_match', 'variance_match'],
    extra: (d) => {
      expect(d.per_run_rates).toHaveLength(30)
      expect(typeof d.mean_match).toBe('boolean')
    },
  },
  {
    path: '/api/coevolution?seed=42',
    fields: ['seed', 'rounds'],
    extra: (d) => expect(d.rounds).toHaveLength(8),
  },
  {
    path: '/api/gossip?seed=42',
    fields: ['seed', 'no_gossip_rate', 'gossip_rate'],
    extra: (d) => {
      expect(d.no_gossip_rate).toBeGreaterThanOrEqual(0)
      expect(d.gossip_rate).toBeLessThanOrEqual(1)
    },
  },
]

test('each endpoint returns JSON with the expected shape', async ({ request }) => {
  for (const { path, fields, extra } of ENDPOINTS) {
    const res = await request.get(path)
    expect(res.status(), path).toBe(200)
    expect(res.headers()['content-type'], path).toContain('application/json')
    const body = await res.json()
    for (const field of fields) {
      expect(body, `${path} missing ${field}`).toHaveProperty(field)
    }
    extra?.(body)
  }
})

test('responses are seeded and reproducible', async ({ request }) => {
  for (const { path } of ENDPOINTS) {
    const [a, b] = await Promise.all([request.get(path), request.get(path)])
    expect(await a.text(), path).toEqual(await b.text())
  }
  // Different seeds should (overwhelmingly likely) differ somewhere.
  const a = await (await request.get('/api/town?seed=1')).text()
  const b = await (await request.get('/api/town?seed=999999')).text()
  expect(a).not.toEqual(b)
})

test('error handling: unknown paths and wrong methods', async ({ request }) => {
  expect((await request.get('/api/nope')).status()).toBe(404)
  expect((await request.get('/definitely-not-a-page')).status()).toBe(404)
  expect((await request.post('/api/town')).status()).toBe(405)
  expect((await request.post('/mcp-not-here')).status()).toBe(404)
})
