import { test, expect } from '@playwright/test'

// Persona 4 — the orchestrator delegating experiments to the A2A agent.

async function rpc(request: import('@playwright/test').APIRequestContext, method: string, params?: unknown) {
  const res = await request.post('/a2a/', { data: { jsonrpc: '2.0', id: 1, method, params } })
  expect(res.ok(), `POST /a2a/ ${method} → ${res.status()}`).toBeTruthy()
  return res.json()
}

test('the agent card is discoverable and complete', async ({ request }) => {
  for (const path of ['/.well-known/agent.json', '/a2a/agent.json']) {
    const res = await request.get(path)
    expect(res.status(), path).toBe(200)
    expect(res.headers()['content-type'], path).toContain('application/json')
    const card = await res.json()
    expect(card.name).toBe('Simulacra Research Agent')
    expect(card.url).toMatch(/\/a2a\/$/)
    expect(card.capabilities.streaming).toBe(false)
    expect(card.skills.length).toBeGreaterThanOrEqual(5)
    for (const skill of card.skills) {
      expect(skill.id).toBeTruthy()
      expect(skill.description.length).toBeGreaterThan(10)
    }
  }
})

test('message/send runs an experiment and returns a completed task', async ({ request }) => {
  const res = await rpc(request, 'message/send', {
    message: { role: 'user', parts: [{ type: 'text', text: 'please run gossip with seed 42' }] },
  })
  const task = res.result
  expect(task.kind).toBe('task')
  expect(task.id).toMatch(/^simulacra-task-\d+$/)
  expect(task.status.state).toBe('completed')
  expect(task.status.message.parts[0].text).toMatch(/gossip/i)

  // The data artifact mirrors the plain JSON API for the same seed.
  const artifact = task.artifacts[0]
  expect(artifact.parts[1].type).toBe('data')
  const viaApi = await (await request.get('/api/gossip?seed=42')).json()
  expect(artifact.parts[1].data).toEqual(viaApi)
})

test('every experiment skill answers through natural language', async ({ request }) => {
  const cases: Array<[string, RegExp, string, string]> = [
    ['run town seed 42', /Town simulation at seed 42/, 'success_rate', '/api/town?seed=42'],
    ['run variance seed 42', /Variance experiment/, 'mean_match', '/api/variance?seed=42'],
    ['run coevolution seed 42', /Co-evolution at seed 42/, 'rounds', '/api/coevolution?seed=42'],
    ['run gossip seed 42', /Gossip experiment at seed 42/, 'gossip_rate', '/api/gossip?seed=42'],
  ]
  for (const [text, summaryRe, probe, apiPath] of cases) {
    const res = await rpc(request, 'message/send', {
      message: { role: 'user', parts: [{ type: 'text', text }] },
    })
    const task = res.result
    expect(task.status.state, text).toBe('completed')
    expect(task.status.message.parts[0].text).toMatch(summaryRe)
    expect(task.artifacts[0].parts[1].data).toHaveProperty(probe)
    expect(task.artifacts[0].parts[1].data).toEqual(await (await request.get(apiPath)).json())
  }
})

test('small talk gets a helpful overview, not a crash', async ({ request }) => {
  const res = await rpc(request, 'message/send', {
    message: { role: 'user', parts: [{ type: 'text', text: 'hello there' }] },
  })
  expect(res.result.status.state).toBe('completed')
  expect(res.result.status.message.parts[0].text).toMatch(/variance/)
})

test('tasks persist: tasks/get returns the created task', async ({ request }) => {
  const sent = await rpc(request, 'message/send', {
    message: { role: 'user', parts: [{ type: 'text', text: 'run town seed 7' }] },
  })
  const id = sent.result.id
  const fetched = await rpc(request, 'tasks/get', { id })
  expect(fetched.result.id).toBe(id)
  expect(fetched.result.status.message.parts[0].text).toMatch(/seed 7/)
})

test('unknown method and unknown task are JSON-RPC errors', async ({ request }) => {
  const unknown = await rpc(request, 'message/stream', {})
  expect(unknown.error.code).toBe(-32601)

  const missing = await rpc(request, 'tasks/get', { id: 'simulacra-task-424242' })
  expect(missing.error.code).toBe(-32001)
})
