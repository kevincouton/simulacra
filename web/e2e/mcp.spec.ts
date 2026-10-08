import { test, expect } from '@playwright/test'

// Persona 3 — the agent builder wiring the MCP tool server into an LLM
// client. Speaks JSON-RPC 2.0 over POST /mcp exactly like a real client.

const TOOLS = ['run_town', 'run_variance', 'run_coevolution', 'run_gossip', 'list_stages', 'get_stage']

async function rpc(request: import('@playwright/test').APIRequestContext, method: string, params?: unknown, raw?: string) {
  const res = await request.post('/mcp', {
    data: raw ?? { jsonrpc: '2.0', id: 1, method, params },
  })
  expect(res.ok(), `POST /mcp ${method} → ${res.status()}`).toBeTruthy()
  expect(res.headers()['content-type']).toContain('application/json')
  return res.json()
}

test('initialize negotiates the protocol and identifies the server', async ({ request }) => {
  const res = await rpc(request, 'initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'e2e', version: '0' },
  })
  expect(res.result.protocolVersion).toBe('2025-06-18')
  expect(res.result.serverInfo.name).toBe('simulacra')
  expect(res.result.capabilities.tools).toBeDefined()
})

test('tools/list advertises the six tools with input schemas', async ({ request }) => {
  const res = await rpc(request, 'tools/list')
  const names = res.result.tools.map((t: any) => t.name)
  expect(names.sort()).toEqual([...TOOLS].sort())
  for (const tool of res.result.tools) {
    expect(tool.description.length, `${tool.name} needs a description`).toBeGreaterThan(10)
    expect(tool.inputSchema.type).toBe('object')
  }
})

test('experiment tools return results consistent with the JSON API', async ({ request }) => {
  const pairs: Array<[string, string, string]> = [
    ['run_town', '/api/town?seed=42', 'success_rate'],
    ['run_variance', '/api/variance?seed=42', 'mean_match'],
    ['run_coevolution', '/api/coevolution?seed=42', 'rounds'],
    ['run_gossip', '/api/gossip?seed=42', 'gossip_rate'],
  ]
  for (const [tool, apiPath, probe] of pairs) {
    const res = await rpc(request, 'tools/call', { name: tool, arguments: { seed: 42 } })
    expect(res.result.isError, tool).toBe(false)
    const viaTool = JSON.parse(res.result.content[0].text)
    const viaApi = await (await request.get(apiPath)).json()
    expect(viaTool, `${tool} vs ${apiPath}`).toEqual(viaApi)
    expect(viaTool).toHaveProperty(probe)
  }
})

test('default seed matches seed 42', async ({ request }) => {
  const withSeed = await rpc(request, 'tools/call', { name: 'run_gossip', arguments: { seed: 42 } })
  const without = await rpc(request, 'tools/call', { name: 'run_gossip' })
  expect(without.result.content).toEqual(withSeed.result.content)
})

test('stage documents are readable as tools', async ({ request }) => {
  const list = await rpc(request, 'tools/call', { name: 'list_stages' })
  const stages = JSON.parse(list.result.content[0].text)
  expect(stages.length).toBeGreaterThanOrEqual(10)
  expect(stages.map((s: any) => s.slug)).toContain('07-subject')

  const doc = await rpc(request, 'tools/call', { name: 'get_stage', arguments: { slug: '01-reward' } })
  expect(doc.result.content[0].text).toContain('# Stage 1')

  const missing = await rpc(request, 'tools/call', { name: 'get_stage', arguments: { slug: '../etc' } })
  expect(missing.result.isError).toBe(true)
})

test('protocol errors follow JSON-RPC codes', async ({ request }) => {
  const parse = await rpc(request, '', undefined, '{not json')
  expect(parse.error.code).toBe(-32700)

  const unknown = await rpc(request, 'bogus/method')
  expect(unknown.error.code).toBe(-32601)

  const badTool = await rpc(request, 'tools/call', { name: 'does_not_exist' })
  expect(badTool.error.code).toBe(-32602)
})
