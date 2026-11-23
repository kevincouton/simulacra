import { test, expect } from '@playwright/test'

const MCP_VERSION = '2026-07-28'
const VERSION_META = 'io.modelcontextprotocol/protocolVersion'
const CAPABILITIES_META = 'io.modelcontextprotocol/clientCapabilities'
const SERVER_INFO_META = 'io.modelcontextprotocol/serverInfo'

async function modernMcp(
  request: import('@playwright/test').APIRequestContext,
  method: string,
  params: Record<string, unknown> = {},
  version = MCP_VERSION,
) {
  const name = typeof params.name === 'string' ? params.name : undefined
  return request.post('/mcp', {
    headers: {
      'MCP-Protocol-Version': version,
      'Mcp-Method': method,
      ...(name ? { 'Mcp-Name': name } : {}),
    },
    data: {
      jsonrpc: '2.0',
      id: 20,
      method,
      params: {
        ...params,
        _meta: {
          [VERSION_META]: version,
          [CAPABILITIES_META]: {},
          'io.modelcontextprotocol/clientInfo': { name: 'playwright', version: '1' },
        },
      },
    },
  })
}

test('modern MCP discovery, caching, and tool calls preserve deterministic results', async ({ request }) => {
  const discovered = await modernMcp(request, 'server/discover')
  expect(discovered.status()).toBe(200)
  const discovery = await discovered.json()
  expect(discovery.result.resultType).toBe('complete')
  expect(discovery.result.supportedVersions).toContain(MCP_VERSION)
  expect(discovery.result.ttlMs).toBeGreaterThan(0)
  expect(discovery.result.cacheScope).toBe('public')
  expect(discovery.result._meta[SERVER_INFO_META].name).toBe('simulacra')

  const listed = await modernMcp(request, 'tools/list')
  expect(listed.status()).toBe(200)
  const tools = await listed.json()
  expect(tools.result.tools).toHaveLength(6)
  expect(tools.result.ttlMs).toBeGreaterThan(0)
  expect(tools.result.cacheScope).toBe('public')

  const called = await modernMcp(request, 'tools/call', { name: 'run_gossip', arguments: { seed: 42 } })
  expect(called.status()).toBe(200)
  const result = await called.json()
  expect(result.result.resultType).toBe('complete')
  expect(result.result.isError).toBe(false)
  expect(JSON.parse(result.result.content[0].text)).toEqual(
    await (await request.get('/api/gossip?seed=42')).json(),
  )
})

test('modern MCP rejects unsupported versions and header/body mismatches', async ({ request }) => {
  const unsupported = await modernMcp(request, 'tools/list', {}, '2099-01-01')
  expect(unsupported.status()).toBe(400)
  const versionError = await unsupported.json()
  expect(versionError.error.code).toBe(-32022)
  expect(versionError.error.data.requested).toBe('2099-01-01')
  expect(versionError.error.data.supported).toContain(MCP_VERSION)

  const mismatch = await request.post('/mcp', {
    headers: { 'MCP-Protocol-Version': '2025-06-18', 'Mcp-Method': 'tools/list' },
    data: {
      jsonrpc: '2.0', id: 21, method: 'tools/list',
      params: { _meta: { [VERSION_META]: MCP_VERSION, [CAPABILITIES_META]: {} } },
    },
  })
  expect(mismatch.status()).toBe(400)
  expect((await mismatch.json()).error.code).toBe(-32020)
})

test('A2A 1.0 card and tasks use current shapes without breaking 0.3 retrieval', async ({ request }) => {
  const cardResponse = await request.get('/.well-known/agent-card.json')
  expect(cardResponse.status()).toBe(200)
  const card = await cardResponse.json()
  expect(card.supportedInterfaces[0].protocolVersion).toBe('1.0')
  expect(card.supportedInterfaces[0].protocolBinding).toBe('JSONRPC')
  expect(card.supportedInterfaces[0].url).toMatch(/\/a2a\/v1$/)
  expect(card.defaultInputModes).toContain('text/plain')
  expect(card.protocolVersion).toBeUndefined()

  const sentResponse = await request.post('/a2a/v1', {
    headers: { 'A2A-Version': '1.0' },
    data: {
      jsonrpc: '2.0', id: 30, method: 'message/send',
      params: { message: { role: 'ROLE_USER', messageId: 'e2e-v1', parts: [{ text: 'run gossip seed 42' }] } },
    },
  })
  expect(sentResponse.status()).toBe(200)
  const sent = await sentResponse.json()
  const sentTask = sent.result.task
  expect(sentTask.kind).toBeUndefined()
  expect(sentTask.status.state).toBe('TASK_STATE_COMPLETED')
  expect(sentTask.status.message.role).toBe('ROLE_AGENT')
  expect(sentTask.status.message.parts[0].type).toBeUndefined()
  expect(sentTask.artifacts[0].parts[1].data).toEqual(
    await (await request.get('/api/gossip?seed=42')).json(),
  )

  const fetchedResponse = await request.post('/a2a/v1', {
    headers: { 'A2A-Version': '1.0' },
    data: { jsonrpc: '2.0', id: 31, method: 'GetTask', params: { id: sentTask.id } },
  })
  const fetched = await fetchedResponse.json()
  expect(fetched.result.id).toBe(sentTask.id)
  expect(fetched.result.status.state).toBe('TASK_STATE_COMPLETED')

  const legacyResponse = await request.post('/a2a/', {
    data: { jsonrpc: '2.0', id: 32, method: 'tasks/get', params: { id: sentTask.id } },
  })
  const legacy = await legacyResponse.json()
  expect(legacy.result.kind).toBe('task')
  expect(legacy.result.status.state).toBe('completed')
})

test('A2A reports unsupported versions with the specified error code', async ({ request }) => {
  const response = await request.post('/a2a/v1', {
    headers: { 'A2A-Version': '2.0' },
    data: { jsonrpc: '2.0', id: 33, method: 'tasks/get', params: { id: 'none' } },
  })
  expect(response.status()).toBe(400)
  const body = await response.json()
  expect(body.error.code).toBe(-32009)
  expect(body.error.data.requested).toBe('2.0')
})
