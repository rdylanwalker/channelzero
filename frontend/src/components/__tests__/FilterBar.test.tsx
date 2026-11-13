import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { ProviderMeta } from '../../types'
import { FilterBar } from '../FilterBar'

const providers: ProviderMeta[] = [
  { id: 'netflix', name: 'Netflix', color: '#e50914' },
  { id: 'hulu', name: 'Hulu', color: '#1ce783' },
]

describe('FilterBar', () => {
  it('renders all available genres in the genre select', () => {
    render(
      <FilterBar
        q=""
        onQChange={vi.fn()}
        providers={[]}
        selectedProvider="all"
        onProviderChange={vi.fn()}
        mediaType="all"
        onMediaTypeChange={vi.fn()}
        releaseKind="all"
        onReleaseKindChange={vi.fn()}
        genres={['Drama']}
        onGenresChange={vi.fn()}
        availableGenres={['Action', 'Comedy', 'Drama', 'Sci-Fi']}
        status="all"
        onStatusChange={vi.fn()}
        minRating="all"
        onMinRatingChange={vi.fn()}
        sort="release_date"
        onSortChange={vi.fn()}
        order="desc"
        onOrderToggle={vi.fn()}
        onResetFilters={vi.fn()}
        totalCount={1}
      />,
    )

    expect(screen.getByRole('button', { name: 'Reset Filters' })).not.toBeDisabled()

    fireEvent.click(screen.getByLabelText('Filter by genres'))
    expect(screen.getByLabelText('Action')).toBeInTheDocument()
    expect(screen.getByLabelText('Comedy')).toBeInTheDocument()
    expect(screen.getByLabelText('Drama')).toBeInTheDocument()
    expect(screen.getByLabelText('Sci-Fi')).toBeInTheDocument()
    expect(screen.getByText('1 program')).toBeInTheDocument()
  })

  it('invokes callbacks for search, selects, sort order, and provider pills', () => {
    const onQChange = vi.fn()
    const onProviderChange = vi.fn()
    const onMediaTypeChange = vi.fn()
    const onReleaseKindChange = vi.fn()
    const onGenresChange = vi.fn()
    const onStatusChange = vi.fn()
    const onMinRatingChange = vi.fn()
    const onSortChange = vi.fn()
    const onOrderToggle = vi.fn()
    const onResetFilters = vi.fn()

    render(
      <FilterBar
        q=""
        onQChange={onQChange}
        providers={providers}
        selectedProvider="netflix"
        onProviderChange={onProviderChange}
        mediaType="all"
        onMediaTypeChange={onMediaTypeChange}
        releaseKind="all"
        onReleaseKindChange={onReleaseKindChange}
        genres={[]}
        onGenresChange={onGenresChange}
        availableGenres={['Action', 'Drama']}
        status="all"
        onStatusChange={onStatusChange}
        minRating="all"
        onMinRatingChange={onMinRatingChange}
        sort="release_date"
        onSortChange={onSortChange}
        order="asc"
        onOrderToggle={onOrderToggle}
        onResetFilters={onResetFilters}
        totalCount={2}
      />,
    )

    expect(screen.getByText('2 programs')).toBeInTheDocument()

    fireEvent.change(screen.getByPlaceholderText('Search titles or synopses...'), {
      target: { value: 'dune' },
    })
    expect(onQChange).toHaveBeenCalledWith('dune')

    fireEvent.change(screen.getByDisplayValue('All Releases'), { target: { value: 'new_title' } })
    expect(onReleaseKindChange).toHaveBeenCalledWith('new_title')

    fireEvent.change(screen.getByDisplayValue('Movies & TV'), { target: { value: 'movie' } })
    expect(onMediaTypeChange).toHaveBeenCalledWith('movie')

    fireEvent.click(screen.getByLabelText('Filter by genres'))
    fireEvent.click(screen.getByLabelText('Action'))
    expect(onGenresChange).toHaveBeenCalledWith(['Action'])

    fireEvent.change(screen.getByDisplayValue('Any Status'), { target: { value: 'watchlist' } })
    expect(onStatusChange).toHaveBeenCalledWith('watchlist')

    fireEvent.change(screen.getByDisplayValue('Any Rating'), { target: { value: '7' } })
    expect(onMinRatingChange).toHaveBeenCalledWith('7')

    fireEvent.change(screen.getByDisplayValue('Release Date'), { target: { value: 'rating' } })
    expect(onSortChange).toHaveBeenCalledWith('rating')

    fireEvent.click(screen.getByLabelText('Toggle sort direction'))
    expect(onOrderToggle).toHaveBeenCalledTimes(1)

    // Clicking active provider resets to 'all'
    fireEvent.click(screen.getByRole('button', { name: 'Netflix' }))
    expect(onProviderChange).toHaveBeenCalledWith('all')

    // Clicking inactive provider selects it
    fireEvent.click(screen.getByRole('button', { name: 'Hulu' }))
    expect(onProviderChange).toHaveBeenCalledWith('hulu')

    // Clicking All Providers
    fireEvent.click(screen.getByRole('button', { name: 'All Providers' }))
    expect(onProviderChange).toHaveBeenCalledWith('all')

    // Clicking Reset Filters
    fireEvent.click(screen.getByRole('button', { name: 'Reset Filters' }))
    expect(onResetFilters).toHaveBeenCalledTimes(1)
  })

  it('disables Reset Filters when all filters and sort are at defaults', () => {
    render(
      <FilterBar
        q=""
        onQChange={vi.fn()}
        providers={providers}
        selectedProvider="all"
        onProviderChange={vi.fn()}
        mediaType="all"
        onMediaTypeChange={vi.fn()}
        releaseKind="all"
        onReleaseKindChange={vi.fn()}
        genres={[]}
        onGenresChange={vi.fn()}
        availableGenres={[]}
        status="all"
        onStatusChange={vi.fn()}
        minRating="all"
        onMinRatingChange={vi.fn()}
        sort="release_date"
        onSortChange={vi.fn()}
        order="desc"
        onOrderToggle={vi.fn()}
        onResetFilters={vi.fn()}
        totalCount={0}
      />,
    )

    expect(screen.getByRole('button', { name: 'Reset Filters' })).toBeDisabled()
  })
})
