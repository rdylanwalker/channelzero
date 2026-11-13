import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { App } from '../../App'
import type {
  Profile,
  ProfileHiddenState,
  ProgramDto,
  PublicConfig,
  SyncStatusResponse,
} from '../../types'

const mockConfig: PublicConfig = {
  region: 'US',
  timezone: 'UTC',
  new_release_days: 30,
  refresh_interval_hours: 6,
  providers: [{ id: 'netflix', name: 'Netflix', color: '#e50914' }],
}

const mockProfiles: Profile[] = [
  {
    id: 'default',
    name: 'Default',
    region: null,
    new_release_days: null,
    created_at: '2026-01-01T00:00:00Z',
  },
  {
    id: 'alice',
    name: 'Alice',
    region: 'GB',
    new_release_days: 14,
    created_at: '2026-01-02T00:00:00Z',
  },
]

const mockPrograms: ProgramDto[] = [
  {
    id: 'prog-1',
    title: 'Severance',
    media_type: 'series',
    release_kind: 'new_episode',
    release_date: '2026-02-10',
    release_year: 2022,
    episode_info: 'S02E04',
    synopsis: 'Office workers.',
    poster_url: null,
    backdrop_url: null,
    genres: ['Sci-Fi'],
    providers: [{ id: 'netflix', name: 'Netflix', url: 'https://netflix.com/1', is_direct: true }],
    primary_url: 'https://netflix.com/1',
    rating: 8.7,
    ratings: [],
    sources: ['TVmaze'],
    imdb_id: null,
    tmdb_id: null,
    tvmaze_id: 1,
    seen: false,
    watched: false,
    watchlist: false,
    not_interested: false,
    last_interacted_at: null,
  },
]

const mockSyncIdle: SyncStatusResponse = {
  last_sync_at: '2026-02-10T12:00:00Z',
  is_syncing: false,
  total_programs: 1,
  refresh_interval_hours: 6,
  new_release_days: 30,
  region: 'US',
  sources: [
    {
      name: 'TVmaze',
      tier: 'anonymous',
      enabled: true,
      configured: true,
      last_sync_at: '2026-02-10T12:00:00Z',
      last_status: 'ok',
      last_error: null,
      items_fetched: 1,
    },
  ],
}

const mockHidden: ProfileHiddenState = {
  hidden_titles: [
    {
      program_id: 'hidden-1',
      title: 'Hidden Movie',
      media_type: 'movie',
      poster_url: null,
      updated_at: '2026-01-05T00:00:00Z',
    },
  ],
  hidden_genres: ['Horror'],
}

function makeFetchMock(overrides?: {
  programs?: ProgramDto[]
  syncStatusSequence?: SyncStatusResponse[]
  profilesList?: Profile[]
}) {
  let syncCallCount = 0
  let profilesList = [...(overrides?.profilesList ?? mockProfiles)]
  let programs = [...(overrides?.programs ?? mockPrograms)]

  return vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === 'string' ? input : input.toString()
    const method = init?.method ?? 'GET'

    const jsonRes = (data: unknown, status = 200) => ({
      ok: true,
      status,
      json: async () => data,
      text: async () => JSON.stringify(data),
    })

    if (url === '/api/config/public') return jsonRes(mockConfig)
    if (url === '/api/profiles' && method === 'GET') return jsonRes(profilesList)
    if (url === '/api/profiles' && method === 'POST') {
      const body = JSON.parse(init?.body as string) as { name: string }
      const created: Profile = {
        id: body.name.toLowerCase(),
        name: body.name,
        region: null,
        new_release_days: null,
        created_at: '2026-02-01T00:00:00Z',
      }
      profilesList = [...profilesList, created]
      return jsonRes(created)
    }
    if (url.startsWith('/api/profiles/') && method === 'PATCH') {
      const id = decodeURIComponent(url.split('/api/profiles/')[1])
      const patch = JSON.parse(init?.body as string) as {
        region: string | null
        new_release_days: number | null
      }
      const existing = profilesList.find((p) => p.id === id) ?? profilesList[0]
      const updated = { ...existing, ...patch }
      profilesList = profilesList.map((p) => (p.id === id ? updated : p))
      return jsonRes(updated)
    }
    if (
      url.startsWith('/api/profiles/') &&
      !url.includes('/interactions/') &&
      !url.includes('/hidden') &&
      method === 'DELETE'
    ) {
      const id = decodeURIComponent(url.split('/api/profiles/')[1])
      profilesList = profilesList.filter((p) => p.id !== id)
      return jsonRes(undefined, 204)
    }
    if (url.startsWith('/api/sync/status')) {
      if (overrides?.syncStatusSequence && overrides.syncStatusSequence.length > 0) {
        const idx = Math.min(syncCallCount, overrides.syncStatusSequence.length - 1)
        syncCallCount++
        return jsonRes(overrides.syncStatusSequence[idx])
      }
      return jsonRes(mockSyncIdle)
    }
    if (url.startsWith('/api/sync/refresh') && method === 'POST') {
      return jsonRes({ started: true, already_running: false })
    }
    if (url.startsWith('/api/programs')) return jsonRes(programs)
    if (url.includes('/hidden-genres') && method === 'POST') return jsonRes(undefined, 204)
    if (url.includes('/hidden-genres/') && method === 'DELETE') return jsonRes(undefined, 204)
    if (url.includes('/hidden') && method === 'GET') return jsonRes(mockHidden)
    if (url.startsWith('/api/genres')) return jsonRes(['Sci-Fi', 'Drama'])
    if (url.includes('/interactions/') && method === 'POST') {
      const body = JSON.parse(init?.body as string) as { not_interested?: boolean }
      if (body.not_interested) {
        const progId = decodeURIComponent(url.split('/interactions/')[1])
        programs = programs.filter((p) => p.id !== progId)
      }
      return jsonRes(undefined, 204)
    }

    return jsonRes({}, 404)
  })
}

describe('App', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('bootstraps, restores saved profile, renders programs, and handles card/detail interactions', async () => {
    localStorage.setItem('channelzero_profile', 'alice')
    const fetchMock = makeFetchMock()
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)

    expect(await screen.findByRole('heading', { name: 'Severance' })).toBeInTheDocument()
    expect(screen.getByText('GB · 14d')).toBeInTheDocument()

    // Open DetailModal
    fireEvent.click(screen.getByRole('button', { name: /Severance/i }))
    expect(await screen.findByText('Office workers.')).toBeInTheDocument()

    // Toggle watched & watchlist from DetailModal (updates selectedProgram & programs)
    fireEvent.click(screen.getByRole('button', { name: 'Mark Watched' }))
    expect(await screen.findByRole('button', { name: 'Watched' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Save to Watchlist' }))
    expect(await screen.findByRole('button', { name: 'In Watchlist' })).toBeInTheDocument()

    // Launch from DetailModal
    const netflixLinks = screen.getAllByRole('link', { name: /Netflix/i })
    fireEvent.click(netflixLinks[netflixLinks.length - 1])

    // Hide genre from DetailModal
    fireEvent.click(screen.getByTitle('Hide all "Sci-Fi" titles from feed'))

    // Not Interested from DetailModal closes modal and removes program
    fireEvent.click(screen.getByRole('button', { name: 'Not Interested' }))
    await waitFor(() => {
      expect(screen.getByText('No new releases found')).toBeInTheDocument()
    })

    // Toggle sort order
    fireEvent.click(screen.getByLabelText('Toggle sort direction'))
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('order=asc'),
        expect.any(Object),
      )
    })
  })

  it('handles sync modal, manual refresh, and sync polling until completion', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    const fetchMock = makeFetchMock({
      syncStatusSequence: [
        mockSyncIdle,
        { ...mockSyncIdle, is_syncing: true },
        { ...mockSyncIdle, is_syncing: false },
      ],
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    await screen.findByRole('heading', { name: 'Severance' })

    // Open SyncStatusModal
    fireEvent.click(screen.getByTitle('API & Sync Status'))
    expect(await screen.findByText('Metadata Sources & Sync Status')).toBeInTheDocument()

    // Trigger refresh -> sets is_syncing: true (both Header and SyncStatusModal show Syncing...)
    fireEvent.click(screen.getByRole('button', { name: /Sync Now/i }))
    expect(await screen.findAllByRole('button', { name: /Syncing.../i })).toHaveLength(2)

    // Advance timer 2s to trigger poll -> sets is_syncing: false
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2100)
    })

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Sync Now/i })).not.toBeDisabled()
    })

    // Close modal
    fireEvent.click(screen.getByLabelText('Close modal'))
    expect(screen.queryByText('Metadata Sources & Sync Status')).not.toBeInTheDocument()
  })

  it('handles ProfileSettingsModal create, update, delete active profile, and unhide actions', async () => {
    const fetchMock = makeFetchMock()
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    await screen.findByRole('heading', { name: 'Severance' })

    fireEvent.click(screen.getByTitle('Profiles & Hidden Titles/Genres'))
    expect(await screen.findByText('Profiles & Preferences')).toBeInTheDocument()

    // Create profile
    fireEvent.change(screen.getByPlaceholderText('New profile name...'), {
      target: { value: 'Bob' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Add' }))
    await waitFor(() => {
      expect(localStorage.getItem('channelzero_profile')).toBe('bob')
    })

    // Update profile settings
    fireEvent.change(screen.getByLabelText('Region (ISO 3166-1)'), { target: { value: 'CA' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await screen.findByText(/Region updated to/i)).toBeInTheDocument()

    // Unhide title and genre
    fireEvent.click(screen.getByTitle('Unhide title'))
    fireEvent.click(screen.getByTitle('Unhide Horror'))

    // Delete active non-default profile (Bob) -> falls back to first profile
    const deleteButtons = screen.getAllByTitle('Delete profile')
    fireEvent.click(deleteButtons[deleteButtons.length - 1])
    await waitFor(() => {
      expect(localStorage.getItem('channelzero_profile')).toBe('default')
    })

    // Close modal
    fireEvent.click(screen.getByLabelText('Close modal'))
  })

  it('logs console.error gracefully when initialization fails', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('Network down')))

    render(<App />)
    await waitFor(() => {
      expect(errorSpy).toHaveBeenCalled()
    })
    errorSpy.mockRestore()
  })

  it('resets filters, resets pagination, and scrolls to top when filters change', async () => {
    const scrollToSpy = vi.fn()
    vi.stubGlobal('scrollTo', scrollToSpy)
    const fetchMock = makeFetchMock()
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    await screen.findByRole('heading', { name: 'Severance' })
    scrollToSpy.mockClear()

    // Change a filter -> scrolls to top
    fireEvent.change(screen.getByDisplayValue('Movies & TV'), { target: { value: 'movie' } })
    await waitFor(() => {
      expect(scrollToSpy).toHaveBeenCalledWith({ top: 0, behavior: 'smooth' })
    })

    const resetBtn = screen.getByRole('button', { name: 'Reset Filters' })
    expect(resetBtn).not.toBeDisabled()

    // Click Reset Filters -> restores default filter and disables Reset Filters
    fireEvent.click(resetBtn)
    await waitFor(() => {
      expect(screen.getByDisplayValue('Movies & TV')).toBeInTheDocument()
      expect(resetBtn).toBeDisabled()
    })
  })

  it('automatically falls back to catalog search when local search returns no results', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    const catalogProgram = {
      ...mockPrograms[0],
      id: 'cat-1',
      title: 'The Legend of Vox Machina',
      release_date: '2022-01-27',
      outside_window: true,
    }

    const baseFetch = makeFetchMock()
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = typeof input === 'string' ? input : input.toString()
      if (url.startsWith('/api/programs/search')) {
        return new Response(JSON.stringify([catalogProgram]), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        })
      }
      if (url.startsWith('/api/programs') && url.includes('q=vox')) {
        return new Response(JSON.stringify([]), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        })
      }
      return baseFetch(input, init)
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    await screen.findByRole('heading', { name: 'Severance' })

    fireEvent.change(screen.getByPlaceholderText('Search titles or synopses...'), {
      target: { value: 'vox' },
    })

    await act(async () => {
      await vi.advanceTimersByTimeAsync(450)
    })

    expect(
      await screen.findByRole('heading', { name: 'The Legend of Vox Machina' }),
    ).toBeInTheDocument()
    expect(screen.getByText('Older than window')).toBeInTheDocument()
    expect(
      screen.getByText(/Showing streaming catalog results older than your release window/i),
    ).toBeInTheDocument()
  })

  it('persists filters in localStorage per profile and restores them when switching profiles', async () => {
    const fetchMock = makeFetchMock()
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    await screen.findByRole('heading', { name: 'Severance' })

    // Select Netflix provider and multi-select Sci-Fi genre on default profile
    fireEvent.click(screen.getByRole('button', { name: 'Netflix' }))
    fireEvent.click(screen.getByLabelText('Filter by genres'))
    fireEvent.click(screen.getByLabelText('Sci-Fi'))

    await waitFor(() => {
      const saved = JSON.parse(localStorage.getItem('channelzero_filters_default') ?? '{}')
      expect(saved.selectedProvider).toBe('netflix')
      expect(saved.genres).toEqual(['Sci-Fi'])
    })

    // Switch to alice -> filters reset to alice's saved/default state
    fireEvent.change(screen.getByLabelText('Active profile'), { target: { value: 'alice' } })
    await waitFor(() => {
      expect(screen.getByLabelText('Filter by genres')).toHaveTextContent('All Genres')
    })

    // Switch back to default -> default's filters are restored
    fireEvent.change(screen.getByLabelText('Active profile'), { target: { value: 'default' } })
    await waitFor(() => {
      expect(screen.getByLabelText('Filter by genres')).toHaveTextContent('Sci-Fi')
    })
  })
})
