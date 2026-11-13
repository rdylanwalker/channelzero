import type {
  Profile,
  ProfileHiddenState,
  ProgramDto,
  PublicConfig,
  SyncStatusResponse,
} from '../types'

async function apiFetch<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
    ...init,
  })
  if (!res.ok) {
    const text = await res.text()
    throw new Error(text || `HTTP ${res.status}`)
  }
  if (res.status === 204) {
    return undefined as T
  }
  return res.json() as Promise<T>
}

export function getPublicConfig(): Promise<PublicConfig> {
  return apiFetch<PublicConfig>('/api/config/public')
}

export interface ProgramFilters {
  profileId: string
  q?: string
  provider?: string
  mediaType?: string
  releaseKind?: string
  genre?: string | string[]
  status?: string
  minRating?: string
  sort?: string
  order?: string
}

function buildProgramQuery(filters: ProgramFilters): URLSearchParams {
  const params = new URLSearchParams()
  params.set('profile_id', filters.profileId)
  if (filters.q) params.set('q', filters.q)
  if (filters.provider && filters.provider !== 'all') params.set('provider', filters.provider)
  if (filters.mediaType && filters.mediaType !== 'all') params.set('media_type', filters.mediaType)
  if (filters.releaseKind && filters.releaseKind !== 'all')
    params.set('release_kind', filters.releaseKind)
  if (Array.isArray(filters.genre)) {
    if (filters.genre.length > 0) params.set('genre', filters.genre.join(','))
  } else if (filters.genre && filters.genre !== 'all') {
    params.set('genre', filters.genre)
  }
  if (filters.status && filters.status !== 'all') params.set('status', filters.status)
  if (filters.minRating && filters.minRating !== 'all') params.set('min_rating', filters.minRating)
  if (filters.sort) params.set('sort', filters.sort)
  if (filters.order) params.set('order', filters.order)
  return params
}

export function listPrograms(filters: ProgramFilters): Promise<ProgramDto[]> {
  return apiFetch<ProgramDto[]>(`/api/programs?${buildProgramQuery(filters).toString()}`)
}

export function searchProgramsCatalog(filters: ProgramFilters): Promise<ProgramDto[]> {
  return apiFetch<ProgramDto[]>(`/api/programs/search?${buildProgramQuery(filters).toString()}`)
}

export function listGenres(profileId: string): Promise<string[]> {
  const params = new URLSearchParams({ profile_id: profileId })
  return apiFetch<string[]>(`/api/genres?${params.toString()}`)
}

export function listProfiles(): Promise<Profile[]> {
  return apiFetch<Profile[]>('/api/profiles')
}

export function createProfile(name: string): Promise<Profile> {
  return apiFetch<Profile>('/api/profiles', {
    method: 'POST',
    body: JSON.stringify({ name }),
  })
}

export function updateProfileSettings(
  id: string,
  patch: { region: string | null; new_release_days: number | null },
): Promise<Profile> {
  return apiFetch<Profile>(`/api/profiles/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  })
}

export function deleteProfile(id: string): Promise<void> {
  return apiFetch<void>(`/api/profiles/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  })
}

export function updateInteraction(
  profileId: string,
  programId: string,
  patch: {
    seen?: boolean
    watched?: boolean
    watchlist?: boolean
    not_interested?: boolean
  },
): Promise<void> {
  return apiFetch<void>(
    `/api/profiles/${encodeURIComponent(profileId)}/interactions/${encodeURIComponent(programId)}`,
    {
      method: 'POST',
      body: JSON.stringify(patch),
    },
  )
}

export function getHiddenState(profileId: string): Promise<ProfileHiddenState> {
  return apiFetch<ProfileHiddenState>(`/api/profiles/${encodeURIComponent(profileId)}/hidden`)
}

export function hideGenre(profileId: string, genre: string): Promise<void> {
  return apiFetch<void>(`/api/profiles/${encodeURIComponent(profileId)}/hidden-genres`, {
    method: 'POST',
    body: JSON.stringify({ genre }),
  })
}

export function unhideGenre(profileId: string, genre: string): Promise<void> {
  return apiFetch<void>(
    `/api/profiles/${encodeURIComponent(profileId)}/hidden-genres/${encodeURIComponent(genre)}`,
    {
      method: 'DELETE',
    },
  )
}

export function getSyncStatus(profileId?: string): Promise<SyncStatusResponse> {
  const query = profileId ? `?profile_id=${encodeURIComponent(profileId)}` : ''
  return apiFetch<SyncStatusResponse>(`/api/sync/status${query}`)
}

export function triggerSyncRefresh(
  profileId?: string,
): Promise<{ started: boolean; already_running: boolean }> {
  const query = profileId ? `?profile_id=${encodeURIComponent(profileId)}` : ''
  return apiFetch<{ started: boolean; already_running: boolean }>(`/api/sync/refresh${query}`, {
    method: 'POST',
  })
}
