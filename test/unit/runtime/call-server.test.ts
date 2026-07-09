import { scheduleActionFlightRefresh } from '@rari/runtime/action-flight-refresh'
import { callServer } from '@rari/runtime/call-server'
import { serializeRouterState } from '@rari/runtime/router-state'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

const flightClientMocks = vi.hoisted(() => ({
  encodeReply: vi.fn<(value: unknown) => Promise<FormData | string>>(
    async (args: unknown) => JSON.stringify(args),
  ),
  createTemporaryReferenceSet: vi.fn(() => new Map()),
  createFromFetch: vi.fn(async () => ({
    a: Promise.resolve({ ok: true }),
    f: Promise.resolve({ type: 'refresh' }),
  })),
}))

vi.mock('virtual:react-flight-client', () => flightClientMocks)

vi.mock('@rari/runtime/action-flight-refresh', () => ({
  scheduleActionFlightRefresh: vi.fn(),
}))

describe('callServer', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
    vi.stubGlobal('window', {
      location: { pathname: '/actions', search: '', href: 'http://localhost/actions' },
    })
    vi.stubGlobal('dispatchEvent', vi.fn())
    flightClientMocks.encodeReply.mockImplementation(
      async (args: unknown) => JSON.stringify(args),
    )
    flightClientMocks.createFromFetch.mockImplementation(async () => ({
      a: Promise.resolve({ ok: true }),
      f: Promise.resolve({ type: 'refresh' }),
    }))
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.clearAllMocks()
  })

  it('posts encodeReply payload with router state and Flight accept header', async () => {
    const fetchMock = vi.mocked(globalThis.fetch)
    fetchMock.mockResolvedValueOnce(new Response('0:{}', {
      status: 200,
      headers: { 'content-type': 'text/x-component' },
    }))

    const result = await callServer('actions/todo-actions_abcd1234#addTodo', [{ text: 'test' }])

    expect(result).toEqual({ ok: true })
    expect(fetchMock).toHaveBeenCalledWith('/actions', expect.objectContaining({
      method: 'POST',
      headers: expect.objectContaining({
        'rsc-action-id': 'actions/todo-actions_abcd1234#addTodo',
        'Accept': 'text/x-component',
        'rari-router-state': serializeRouterState(),
        'Content-Type': 'text/plain;charset=UTF-8',
      }),
    }))
    expect(flightClientMocks.createFromFetch).toHaveBeenCalled()
    expect(scheduleActionFlightRefresh).toHaveBeenCalledWith(
      expect.any(Response),
      expect.objectContaining({ a: expect.anything(), f: expect.anything() }),
      { ok: true },
    )
  })

  it('passes multipart FormData without forcing text/plain content type', async () => {
    const fetchMock = vi.mocked(globalThis.fetch)
    const formData = new FormData()
    formData.append('text', 'todo')

    flightClientMocks.encodeReply.mockResolvedValueOnce(formData)

    fetchMock.mockResolvedValueOnce(new Response('0:{}', {
      status: 200,
      headers: { 'content-type': 'text/x-component' },
    }))

    await callServer('actions/todo-actions_abcd1234#addTodo', [null, formData])

    expect(fetchMock).toHaveBeenCalledWith('/actions', expect.objectContaining({
      method: 'POST',
      body: formData,
      headers: expect.objectContaining({
        'rsc-action-id': 'actions/todo-actions_abcd1234#addTodo',
        'Accept': 'text/x-component',
      }),
    }))

    const requestInit = fetchMock.mock.calls[0]![1] as RequestInit
    const headers = requestInit.headers as Record<string, string>
    expect(headers['Content-Type']).toBeUndefined()
  })

  it('follows x-action-redirect without decoding Flight', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValueOnce(new Response('', {
      status: 200,
      headers: {
        'content-type': 'text/plain',
        'x-action-redirect': '/actions;push',
      },
    }))

    const result = await callServer('actions/todo-actions_abcd1234#addTodo', [])

    expect(result).toEqual({ redirect: '/actions' })
    expect(flightClientMocks.createFromFetch).not.toHaveBeenCalled()
    expect(globalThis.dispatchEvent).not.toHaveBeenCalled()
  })

  it('throws when the server returns a plain text error', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValueOnce(new Response('boom', {
      status: 400,
      headers: { 'content-type': 'text/plain;charset=UTF-8' },
    }))

    await expect(callServer('actions/todo-actions_abcd1234#addTodo', [])).rejects.toThrow('boom')
  })

  it('follows x-action-redirect and updates window.location', async () => {
    const location = { pathname: '/actions', search: '', href: 'http://localhost/actions' }
    vi.stubGlobal('window', { location })

    vi.mocked(globalThis.fetch).mockResolvedValueOnce(new Response('', {
      status: 200,
      headers: {
        'content-type': 'text/plain',
        'x-action-redirect': '/dashboard;push',
      },
    }))

    const result = await callServer('actions/todo-actions_abcd1234#addTodo', [])

    expect(result).toEqual({ redirect: '/dashboard' })
    expect(location.href).toBe('http://localhost/dashboard')
  })

  it('ignores unsafe redirect protocols', async () => {
    const location = { pathname: '/actions', search: '', href: 'http://localhost/actions' }
    vi.stubGlobal('window', { location })

    vi.mocked(globalThis.fetch).mockResolvedValueOnce(new Response('', {
      status: 200,
      headers: {
        'content-type': 'text/plain',
        'x-action-redirect': 'javascript:alert(1);push',
      },
    }))

    const result = await callServer('actions/todo-actions_abcd1234#addTodo', [])

    expect(result).toEqual({ redirect: 'javascript:alert(1)' })
    expect(location.href).toBe('http://localhost/actions')
  })

  it('throws when the server returns a non-flight 500 error', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValueOnce(new Response('', {
      status: 500,
      statusText: 'Internal Server Error',
      headers: { 'content-type': 'text/html' },
    }))

    await expect(callServer('actions/todo-actions_abcd1234#addTodo', []))
      .rejects
      .toThrow('Server action "actions/todo-actions_abcd1234#addTodo" failed with status 500: Internal Server Error')
  })
})
