export type MediaType = 'movie' | 'series'
export type ReleaseKind = 'new_title' | 'new_episode'

export interface ProviderLink {
  id: string
  name: string
  url: string
  is_direct: boolean
}

export interface SourceRating {
  source: string
  score: string
  normalized_10: number | null
}

export interface ProgramDto {
  id: string
  title: string
  media_type: MediaType
  release_kind: ReleaseKind
  release_date: string
  release_year: number | null
  episode_info: string | null
  synopsis: string | null
  poster_url: string | null
  backdrop_url: string | null
  genres: string[]
  providers: ProviderLink[]
  primary_url: string
  rating: number | null
  ratings: SourceRating[]
  sources: string[]
  imdb_id: string | null
  tmdb_id: number | null
  tvmaze_id: number | null
  seen: boolean
  watched: boolean
  watchlist: boolean
  not_interested: boolean
  last_interacted_at: string | null
}

export interface Profile {
  id: string
  name: string
  region: string | null
  new_release_days: number | null
  created_at: string
}

export interface HiddenItem {
  program_id: string
  title: string
  media_type: string
  poster_url: string | null
  updated_at: string
}

export interface ProfileHiddenState {
  hidden_titles: HiddenItem[]
  hidden_genres: string[]
}

export interface SourceStatus {
  name: string
  tier: string
  enabled: boolean
  configured: boolean
  last_sync_at: string | null
  last_status: string
  last_error: string | null
  items_fetched: number
}

export interface SyncStatusResponse {
  last_sync_at: string | null
  is_syncing: boolean
  total_programs: number
  refresh_interval_hours: number
  new_release_days: number
  region: string
  sources: SourceStatus[]
}

export interface ProviderMeta {
  id: string
  name: string
  color: string
}

export interface PublicConfig {
  region: string
  timezone: string
  new_release_days: number
  refresh_interval_hours: number
  providers: ProviderMeta[]
}
