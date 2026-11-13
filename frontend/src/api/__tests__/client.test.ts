import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  createProfile,
  deleteProfile,
  getHiddenState,
  getPublicConfig,
  getSyncStatus,
  hideGenre,
  listGenres,
  listProfiles,
  listPrograms,
  triggerSyncRefresh,
  unhideGenre,
  updateInteraction,
  updateProfileSettings,
} from '../client'

function mockFetchResponse(options: {
  ok?: boolean
  status?: number
  json?: unknown
  text?: string
}) {
  const { ok = true, status = 200, json = {}, text = '' } = options
  const fetchMock = vi.fn().mockResolvedValue({
    ok,
    status,
    json: vi.fn().mockResolvedValue(json),
    text: vi.fn().mockResolvedValue(text),
  })
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

describe('api/client', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('getPublicConfig fetches /api/config/public', async () => {
    const payload = {
      region: 'US',
      timezone: 'UTC',
      new_release_days: 30,
      refresh_interval_hours: 6,
      providers: [],
    }
    const fetchMock = mockFetchResponse({ json: payload })

    const res = await getPublicConfig()
    expect(res).toEqual(payload)
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/config/public',
      expect.objectContaining({
        headers: expect.objectContaining({ 'Content-Type': 'application/json' }),
      }),
    )
  })

  it('listPrograms builds query params and omits "all" filters', async () => {
    const fetchMock = mockFetchResponse({ json: [] })

    await listPrograms({
      profileId: 'default',
      provider: 'all',
      mediaType: 'all',
      releaseKind: 'all',
      genre: 'all',
      status: 'all',
      minRating: 'all',
    })

    expect(fetchMock).toHaveBeenCalledWith('/api/programs?profile_id=default', expect.any(Object))
  })

  it('listPrograms includes all active filter query params', async () => {
    const fetchMock = mockFetchResponse({ json: [] })

    await listPrograms({
      profileId: 'p1',
      q: 'severance',
      provider: 'apple_tv',
      mediaType: 'series',
      releaseKind: 'new_episode',
      genre: 'Sci-Fi',
      status: 'unseen',
      minRating: '7',
      sort: 'rating',
      order: 'asc',
    })

    const calledUrl = fetchMock.mock.calls[0][0] as string
    const params = new URLSearchParams(calledUrl.split('?')[1])
    expect(params.get('profile_id')).toBe('p1')
    expect(params.get('q')).toBe('severance')
    expect(params.get('provider')).toBe('apple_tv')
    expect(params.get('media_type')).toBe('series')
    expect(params.get('release_kind')).toBe('new_episode')
    expect(params.get('genre')).toBe('Sci-Fi')
    expect(params.get('status')).toBe('unseen')
    expect(params.get('min_rating')).toBe('7')
    expect(params.get('sort')).toBe('rating')
    expect(params.get('order')).toBe('asc')
  })

  it('listGenres fetches /api/genres with profile_id', async () => {
    const fetchMock = mockFetchResponse({ json: ['Drama', 'Comedy'] })
    const res = await listGenres('p1')
    expect(res).toEqual(['Drama', 'Comedy'])
    expect(fetchMock).toHaveBeenCalledWith('/api/genres?profile_id=p1', expect.any(Object))
  })

  it('listProfiles fetches /api/profiles', async () => {
    const fetchMock = mockFetchResponse({ json: [] })
    await listProfiles()
    expect(fetchMock).toHaveBeenCalledWith('/api/profiles', expect.any(Object))
  })

  it('createProfile posts JSON body', async () => {
    const fetchMock = mockFetchResponse({ json: { id: 'alice', name: 'Alice' } })
    const res = await createProfile('Alice')
    expect(res).toEqual({ id: 'alice', name: 'Alice' })
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/profiles',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ name: 'Alice' }),
      }),
    )
  })

  it('updateProfileSettings sends PATCH with encoded id', async () => {
    const fetchMock = mockFetchResponse({ json: { id: 'p 1', region: 'GB', new_release_days: 14 } })
    await updateProfileSettings('p 1', { region: 'GB', new_release_days: 14 })
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/profiles/p%201',
      expect.objectContaining({
        method: 'PATCH',
        body: JSON.stringify({ region: 'GB', new_release_days: 14 }),
      }),
    )
  })

  it('deleteProfile handles 204 No Content', async () => {
    const fetchMock = mockFetchResponse({ status: 204 })
    const res = await deleteProfile('p1')
    expect(res).toBeUndefined()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/profiles/p1',
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('updateInteraction posts interaction patch with encoded ids', async () => {
    const fetchMock = mockFetchResponse({ status: 204 })
    await updateInteraction('p1', 'prog/1', { watched: true, seen: true })
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/profiles/p1/interactions/prog%2F1',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ watched: true, seen: true }),
      }),
    )
  })

  it('getHiddenState fetches hidden state', async () => {
    const fetchMock = mockFetchResponse({ json: { hidden_titles: [], hidden_genres: [] } })
    const res = await getHiddenState('p1')
    expect(res).toEqual({ hidden_titles: [], hidden_genres: [] })
    expect(fetchMock).toHaveBeenCalledWith('/api/profiles/p1/hidden', expect.any(Object))
  })

  it('hideGenre and unhideGenre call respective endpoints', async () => {
    const fetchMock = mockFetchResponse({ status: 204 })
    await hideGenre('p1', 'Sci-Fi')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/profiles/p1/hidden-genres',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ genre: 'Sci-Fi' }),
      }),
    )

    await unhideGenre('p1', 'Sci-Fi')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/profiles/p1/hidden-genres/Sci-Fi',
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('getSyncStatus and triggerSyncRefresh handle optional profileId', async () => {
    const fetchMock = mockFetchResponse({ json: { is_syncing: false } })
    await getSyncStatus()
    expect(fetchMock).toHaveBeenCalledWith('/api/sync/status', expect.any(Object))

    await getSyncStatus('p1')
    expect(fetchMock).toHaveBeenCalledWith('/api/sync/status?profile_id=p1', expect.any(Object))

    await triggerSyncRefresh()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/sync/refresh',
      expect.objectContaining({ method: 'POST' }),
    )

    await triggerSyncRefresh('p1')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/sync/refresh?profile_id=p1',
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('throws error message from response body or fallback HTTP status', async () => {
    mockFetchResponse({ ok: false, status: 400, text: 'Bad request payload' })
    await expect(getPublicConfig()).rejects.toThrow('Bad request payload')

    mockFetchResponse({ ok: false, status: 500, text: '' })
    await expect(getPublicConfig()).rejects.toThrow('HTTP 500')
  })
})
