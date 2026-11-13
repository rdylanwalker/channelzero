import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { ProgramDto, ProviderMeta } from '../../types'
import { DetailModal } from '../DetailModal'

const providerMetaMap = new Map<string, ProviderMeta>([
  ['netflix', { id: 'netflix', name: 'Netflix', color: '#e50914' }],
])

const baseProgram: ProgramDto = {
  id: 'prog-1',
  title: 'Severance',
  media_type: 'series',
  release_kind: 'new_episode',
  release_date: '2026-02-10',
  release_year: 2022,
  episode_info: 'S02E04 · Woe Hollow',
  synopsis: 'Mark leads a team of office workers.',
  poster_url: 'https://example.com/poster.jpg',
  backdrop_url: null,
  genres: ['Sci-Fi', 'Thriller'],
  providers: [
    { id: 'netflix', name: 'Netflix', url: 'https://netflix.com/watch/1', is_direct: true },
    { id: 'unknown', name: 'OtherStream', url: 'https://other.com/search', is_direct: false },
  ],
  primary_url: 'https://netflix.com/watch/1',
  rating: 8.7,
  ratings: [
    { source: 'IMDb', score: '8.7/10', normalized_10: 8.7 },
    { source: 'TMDB', score: '84%', normalized_10: 8.4 },
  ],
  sources: ['TMDB', 'OMDb'],
  imdb_id: 'tt11280740',
  tmdb_id: 95396,
  tvmaze_id: null,
  seen: false,
  watched: false,
  watchlist: false,
  not_interested: false,
  last_interacted_at: null,
}

describe('DetailModal', () => {
  it('renders nothing when program is null', () => {
    const { container } = render(
      <DetailModal
        program={null}
        providerMetaMap={providerMetaMap}
        onClose={vi.fn()}
        onLaunch={vi.fn()}
        onToggleWatched={vi.fn()}
        onToggleWatchlist={vi.fn()}
        onNotInterested={vi.fn()}
        onHideGenre={vi.fn()}
      />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('renders program details, ratings, genres, providers, and handles actions', () => {
    const onClose = vi.fn()
    const onLaunch = vi.fn()
    const onToggleWatched = vi.fn()
    const onToggleWatchlist = vi.fn()
    const onNotInterested = vi.fn()
    const onHideGenre = vi.fn()

    render(
      <DetailModal
        program={baseProgram}
        providerMetaMap={providerMetaMap}
        onClose={onClose}
        onLaunch={onLaunch}
        onToggleWatched={onToggleWatched}
        onToggleWatchlist={onToggleWatchlist}
        onNotInterested={onNotInterested}
        onHideGenre={onHideGenre}
      />,
    )

    expect(screen.getByText('Severance')).toBeInTheDocument()
    expect(screen.getByAltText('Severance')).toHaveAttribute(
      'src',
      'https://example.com/poster.jpg',
    )
    expect(screen.getByText('New Episode')).toBeInTheDocument()
    expect(screen.getByText('S02E04 · Woe Hollow')).toBeInTheDocument()
    expect(screen.getByText('IMDb:')).toBeInTheDocument()
    expect(screen.getByText('8.7/10')).toBeInTheDocument()
    expect(screen.getByText('Mark leads a team of office workers.')).toBeInTheDocument()
    expect(screen.getByText('Direct link')).toBeInTheDocument()
    expect(screen.getByText('Provider search')).toBeInTheDocument()
    expect(screen.getByText('Metadata sources: TMDB, OMDb')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'IMDb Page' })).toHaveAttribute(
      'href',
      'https://www.imdb.com/title/tt11280740/',
    )

    fireEvent.click(screen.getByTitle('Hide all "Sci-Fi" titles from feed'))
    expect(onHideGenre).toHaveBeenCalledWith('Sci-Fi')

    fireEvent.click(screen.getByRole('button', { name: 'Mark Watched' }))
    expect(onToggleWatched).toHaveBeenCalledWith(baseProgram)

    fireEvent.click(screen.getByRole('button', { name: 'Save to Watchlist' }))
    expect(onToggleWatchlist).toHaveBeenCalledWith(baseProgram)

    fireEvent.click(screen.getByRole('link', { name: /Netflix/i }))
    expect(onLaunch).toHaveBeenCalledWith(baseProgram)

    fireEvent.click(screen.getByRole('button', { name: 'Not Interested' }))
    expect(onNotInterested).toHaveBeenCalledWith(baseProgram)
    expect(onClose).toHaveBeenCalledTimes(1)

    fireEvent.keyDown(window, { key: 'Escape' })
    expect(onClose).toHaveBeenCalledTimes(2)
  })

  it('renders movie fallback without poster, watched/watchlist active labels, and empty sources', () => {
    const movieProgram: ProgramDto = {
      ...baseProgram,
      media_type: 'movie',
      release_kind: 'new_title',
      poster_url: null,
      episode_info: null,
      synopsis: null,
      ratings: [],
      genres: [],
      sources: [],
      imdb_id: null,
      watched: true,
      watchlist: true,
    }

    render(
      <DetailModal
        program={movieProgram}
        providerMetaMap={providerMetaMap}
        onClose={vi.fn()}
        onLaunch={vi.fn()}
        onToggleWatched={vi.fn()}
        onToggleWatchlist={vi.fn()}
        onNotInterested={vi.fn()}
        onHideGenre={vi.fn()}
      />,
    )

    expect(screen.getByText('New Series / Movie')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Watched' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'In Watchlist' })).toBeInTheDocument()
    expect(screen.getByText('Metadata sources: TVmaze')).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'IMDb Page' })).not.toBeInTheDocument()
  })
})
