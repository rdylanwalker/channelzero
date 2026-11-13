import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { ProgramDto, ProviderMeta } from '../../types'
import { ProgramCard } from '../ProgramCard'

const providerMetaMap = new Map<string, ProviderMeta>([
  ['netflix', { id: 'netflix', name: 'Netflix', color: '#e50914' }],
])

const baseProgram: ProgramDto = {
  id: 'prog-1',
  title: 'Dune: Part Two',
  media_type: 'movie',
  release_kind: 'new_title',
  release_date: '2026-01-15',
  release_year: 2024,
  episode_info: null,
  synopsis: 'Paul Atreides unites with the Fremen.',
  poster_url: 'https://example.com/dune.jpg',
  backdrop_url: null,
  genres: ['Sci-Fi'],
  providers: [
    { id: 'netflix', name: 'Netflix', url: 'https://netflix.com/dune', is_direct: true },
    { id: 'max', name: 'Max', url: 'https://max.com/search?q=dune', is_direct: false },
  ],
  primary_url: 'https://netflix.com/dune',
  rating: 8.6,
  ratings: [],
  sources: ['TMDB'],
  imdb_id: null,
  tmdb_id: 693134,
  tvmaze_id: null,
  seen: false,
  watched: false,
  watchlist: false,
  not_interested: false,
  last_interacted_at: null,
}

describe('ProgramCard', () => {
  it('renders poster, rating, providers, and handles all click actions', () => {
    const onLaunch = vi.fn()
    const onOpenDetail = vi.fn()
    const onToggleWatched = vi.fn()
    const onToggleWatchlist = vi.fn()
    const onNotInterested = vi.fn()

    render(
      <ProgramCard
        program={baseProgram}
        providerMetaMap={providerMetaMap}
        onLaunch={onLaunch}
        onOpenDetail={onOpenDetail}
        onToggleWatched={onToggleWatched}
        onToggleWatchlist={onToggleWatchlist}
        onNotInterested={onNotInterested}
      />,
    )

    expect(screen.getByAltText('Dune: Part Two')).toHaveAttribute(
      'src',
      'https://example.com/dune.jpg',
    )
    expect(screen.getByText('8.6')).toBeInTheDocument()
    expect(screen.getByTitle('Open on Netflix (direct link)')).toBeInTheDocument()
    expect(screen.getByTitle('Open on Max (search)')).toBeInTheDocument()

    // Launch via poster link
    fireEvent.click(screen.getByTitle('Watch Dune: Part Two on Netflix'))
    expect(onLaunch).toHaveBeenCalledTimes(1)

    // Launch via provider badge
    fireEvent.click(screen.getByTitle('Open on Max (search)'))
    expect(onLaunch).toHaveBeenCalledTimes(2)

    // Open detail modal via title button
    fireEvent.click(screen.getByRole('button', { name: /Dune: Part Two/i }))
    expect(onOpenDetail).toHaveBeenCalledWith(baseProgram)

    // Footer actions
    fireEvent.click(screen.getByTitle('Mark as watched'))
    expect(onToggleWatched).toHaveBeenCalledWith(baseProgram)

    fireEvent.click(screen.getByTitle('Save to watchlist'))
    expect(onToggleWatchlist).toHaveBeenCalledWith(baseProgram)

    fireEvent.click(screen.getByTitle('Not interested (hide title)'))
    expect(onNotInterested).toHaveBeenCalledWith(baseProgram)
  })

  it('renders fallback poster, seen/watchlist badges, and episode info for series', () => {
    const seenSeries: ProgramDto = {
      ...baseProgram,
      title: 'Andor',
      media_type: 'series',
      poster_url: null,
      rating: null,
      episode_info: 'S02E01',
      providers: [],
      seen: true,
      watched: false,
      watchlist: true,
    }

    render(
      <ProgramCard
        program={seenSeries}
        providerMetaMap={providerMetaMap}
        onLaunch={vi.fn()}
        onOpenDetail={vi.fn()}
        onToggleWatched={vi.fn()}
        onToggleWatchlist={vi.fn()}
        onNotInterested={vi.fn()}
      />,
    )

    expect(screen.getByTitle('Watch Andor on streaming service')).toBeInTheDocument()
    expect(screen.getByText('Seen')).toBeInTheDocument()
    expect(screen.getByText('Saved')).toBeInTheDocument()
    expect(screen.getByText('S02E01')).toBeInTheDocument()
    expect(screen.getByTitle('Remove from watchlist')).toBeInTheDocument()
  })

  it('renders Watched badge and unmark title when watched is true', () => {
    const watchedMovie: ProgramDto = {
      ...baseProgram,
      poster_url: null,
      watched: true,
      seen: true,
    }

    render(
      <ProgramCard
        program={watchedMovie}
        providerMetaMap={providerMetaMap}
        onLaunch={vi.fn()}
        onOpenDetail={vi.fn()}
        onToggleWatched={vi.fn()}
        onToggleWatchlist={vi.fn()}
        onNotInterested={vi.fn()}
      />,
    )

    expect(screen.getByText('Watched')).toBeInTheDocument()
    expect(screen.queryByText('Seen')).not.toBeInTheDocument()
    expect(screen.getByTitle('Unmark watched')).toBeInTheDocument()
  })

  it('renders Older than window badge when outside_window is true', () => {
    render(
      <ProgramCard
        program={{ ...baseProgram, outside_window: true }}
        providerMetaMap={providerMetaMap}
        onLaunch={vi.fn()}
        onOpenDetail={vi.fn()}
        onToggleWatched={vi.fn()}
        onToggleWatchlist={vi.fn()}
        onNotInterested={vi.fn()}
      />,
    )

    expect(screen.getByText('Older than window')).toBeInTheDocument()
  })
})
