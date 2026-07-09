import { callServer } from '@rari/runtime/call-server'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

const flightClientMocks = vi.hoisted(() => ({
  encodeReply: vi.fn<(value: unknown) => Promise<FormData | string>>(
    async (args: unknown) => JSON.stringify(args),
  ),
}))

vi.mock('virtual:react-flight-client', () => flightClientMocks)

describe('callServer', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
    flightClientMocks.encodeReply.mockImplementation(
      async (args: unknown) => JSON.stringify(args),
    )
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.clearAllMocks()
  })

  it('posts encodeReply payload with rsc-action-id header', async () => {
    const fetchMock = vi.mocked(globalThis.fetch)
    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify({
      success: true,
      result: { ok: true },
    }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    }))

    const result = await callServer('actions/todo-actions_abcd1234#addTodo', [{ text: 'test' }])

    expect(result).toEqual({ ok: true })
    expect(fetchMock).toHaveBeenCalledWith('/_rari/action', expect.objectContaining({
      method: 'POST',
      headers: expect.objectContaining({
        'rsc-action-id': 'actions/todo-actions_abcd1234#addTodo',
        'Content-Type': 'text/plain;charset=UTF-8',
      }),
    }))
  })

  it('passes multipart FormData without forcing text/plain content type', async () => {
    const fetchMock = vi.mocked(globalThis.fetch)
    const formData = new FormData()
    formData.append('text', 'todo')

    flightClientMocks.encodeReply.mockResolvedValueOnce(formData)

    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify({
      success: true,
      result: { ok: true },
    }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    }))

    await callServer('actions/todo-actions_abcd1234#addTodo', [null, formData])

    expect(fetchMock).toHaveBeenCalledWith('/_rari/action', expect.objectContaining({
      method: 'POST',
      body: formData,
      headers: expect.objectContaining({
        'rsc-action-id': 'actions/todo-actions_abcd1234#addTodo',
      }),
    }))

    const requestInit = fetchMock.mock.calls[0]![1] as RequestInit
    const headers = requestInit.headers as Record<string, string>
    expect(headers['Content-Type']).toBeUndefined()
  })

  it('throws when the server returns a JSON error', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValueOnce(new Response(JSON.stringify({
      success: false,
      error: 'boom',
    }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    }))

    await expect(callServer('actions/todo-actions_abcd1234#addTodo', [])).rejects.toThrow('boom')
  })
})
