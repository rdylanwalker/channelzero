import { useEffect, useRef, useState } from 'react'
import { ArrowDownUp, Check, ChevronDown, RotateCcw, Search } from 'lucide-react'
import type { ProviderMeta } from '../types'
import { Button } from './ui/Button'
import { Input } from './ui/Input'
import { Select } from './ui/Select'

interface FilterBarProps {
  q: string
  onQChange: (v: string) => void
  providers: ProviderMeta[]
  selectedProvider: string
  onProviderChange: (v: string) => void
  mediaType: string
  onMediaTypeChange: (v: string) => void
  releaseKind: string
  onReleaseKindChange: (v: string) => void
  genres: string[]
  onGenresChange: (v: string[]) => void
  availableGenres: string[]
  status: string
  onStatusChange: (v: string) => void
  minRating: string
  onMinRatingChange: (v: string) => void
  sort: string
  onSortChange: (v: string) => void
  order: 'asc' | 'desc'
  onOrderToggle: () => void
  onResetFilters: () => void
  totalCount: number
}

export function FilterBar({
  q,
  onQChange,
  providers,
  selectedProvider,
  onProviderChange,
  mediaType,
  onMediaTypeChange,
  releaseKind,
  onReleaseKindChange,
  genres,
  onGenresChange,
  availableGenres,
  status,
  onStatusChange,
  minRating,
  onMinRatingChange,
  sort,
  onSortChange,
  order,
  onOrderToggle,
  onResetFilters,
  totalCount,
}: FilterBarProps) {
  const [genreOpen, setGenreOpen] = useState(false)
  const genreMenuRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    if (!genreOpen) return
    const handleMouseDown = (e: MouseEvent) => {
      if (genreMenuRef.current && !genreMenuRef.current.contains(e.target as Node)) {
        setGenreOpen(false)
      }
    }
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setGenreOpen(false)
      }
    }
    document.addEventListener('mousedown', handleMouseDown)
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('mousedown', handleMouseDown)
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [genreOpen])

  const toggleGenre = (g: string) => {
    if (genres.includes(g)) {
      onGenresChange(genres.filter((item) => item !== g))
    } else {
      onGenresChange([...genres, g])
    }
  }

  const genreLabel =
    genres.length === 0
      ? 'All Genres'
      : genres.length === 1
        ? genres[0]
        : `Genres (${genres.length})`

  const hasActiveFilters =
    q !== '' ||
    selectedProvider !== 'all' ||
    mediaType !== 'all' ||
    releaseKind !== 'all' ||
    genres.length > 0 ||
    status !== 'all' ||
    minRating !== 'all' ||
    sort !== 'release_date' ||
    order !== 'desc'

  return (
    <section className="space-y-3 rounded-xl border border-border bg-card/90 p-4 backdrop-blur-md">
      <div className="flex flex-col gap-3 md:flex-row md:items-center">
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            value={q}
            onChange={(e) => onQChange(e.target.value)}
            placeholder="Search titles or synopses..."
            className="pl-9"
          />
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Select value={releaseKind} onChange={(e) => onReleaseKindChange(e.target.value)}>
            <option value="all">All Releases</option>
            <option value="new_title">New Series / Movies</option>
            <option value="new_episode">New Episodes</option>
          </Select>

          <Select value={mediaType} onChange={(e) => onMediaTypeChange(e.target.value)}>
            <option value="all">Movies &amp; TV</option>
            <option value="movie">Movies</option>
            <option value="series">TV Series</option>
          </Select>

          <div className="relative" ref={genreMenuRef}>
            <button
              type="button"
              onClick={() => setGenreOpen((prev) => !prev)}
              aria-label="Filter by genres"
              aria-expanded={genreOpen}
              className="flex h-9 items-center justify-between gap-2 rounded-md border border-border bg-muted/80 px-2.5 py-1 text-sm text-foreground transition hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              <span className="truncate max-w-[120px]">{genreLabel}</span>
              <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
            </button>

            {genreOpen && (
              <div className="absolute left-0 top-full z-40 mt-1 w-52 rounded-lg border border-border bg-card p-1.5 shadow-xl">
                <div className="flex items-center justify-between border-b border-border/60 px-2 pb-1.5 mb-1">
                  <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                    Genres
                  </span>
                  {genres.length > 0 && (
                    <button
                      type="button"
                      onClick={() => onGenresChange([])}
                      className="text-[11px] text-accent hover:underline"
                    >
                      Clear
                    </button>
                  )}
                </div>
                <div className="max-h-56 overflow-y-auto space-y-0.5">
                  {availableGenres.length === 0 ? (
                    <p className="px-2 py-1.5 text-xs text-muted-foreground">No genres available</p>
                  ) : (
                    availableGenres.map((g) => {
                      const checked = genres.includes(g)
                      return (
                        <label
                          key={g}
                          className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-xs text-foreground hover:bg-muted"
                        >
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={() => toggleGenre(g)}
                            className="sr-only"
                          />
                          <span
                            className={`flex h-4 w-4 items-center justify-center rounded border ${
                              checked
                                ? 'border-accent bg-accent text-white'
                                : 'border-border bg-background'
                            }`}
                          >
                            {checked && <Check className="h-3 w-3" />}
                          </span>
                          <span className="truncate">{g}</span>
                        </label>
                      )
                    })
                  )}
                </div>
              </div>
            )}
          </div>

          <Select value={status} onChange={(e) => onStatusChange(e.target.value)}>
            <option value="all">Any Status</option>
            <option value="unseen">Unseen</option>
            <option value="seen">Seen</option>
            <option value="watched">Watched</option>
            <option value="watchlist">Watchlist</option>
          </Select>

          <Select value={minRating} onChange={(e) => onMinRatingChange(e.target.value)}>
            <option value="all">Any Rating</option>
            <option value="6">6.0+ / 10</option>
            <option value="7">7.0+ / 10</option>
            <option value="8">8.0+ / 10</option>
          </Select>

          <div className="flex items-center gap-1">
            <Select value={sort} onChange={(e) => onSortChange(e.target.value)}>
              <option value="release_date">Release Date</option>
              <option value="rating">Rating</option>
              <option value="title">Title</option>
            </Select>
            <Button
              variant="outline"
              size="icon"
              onClick={onOrderToggle}
              title={order === 'desc' ? 'Descending' : 'Ascending'}
              aria-label="Toggle sort direction"
            >
              <ArrowDownUp className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>

      {/* Provider filter pills */}
      <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
        <div className="flex flex-wrap items-center gap-1.5">
          <button
            onClick={() => onProviderChange('all')}
            className={`rounded-full px-3 py-1 text-xs font-medium transition ${
              selectedProvider === 'all'
                ? 'bg-accent text-white'
                : 'bg-muted text-muted-foreground hover:text-foreground'
            }`}
          >
            All Providers
          </button>
          {providers.map((p) => {
            const active = selectedProvider === p.id
            return (
              <button
                key={p.id}
                onClick={() => onProviderChange(active ? 'all' : p.id)}
                className={`flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs font-medium transition ${
                  active
                    ? 'border-transparent text-white shadow-sm'
                    : 'border-border bg-muted/40 text-muted-foreground hover:text-foreground'
                }`}
                style={active ? { backgroundColor: p.color } : undefined}
              >
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ backgroundColor: active ? '#fff' : p.color }}
                />
                {p.name}
              </button>
            )
          })}
        </div>

        <div className="flex items-center gap-3">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onResetFilters}
            disabled={!hasActiveFilters}
          >
            <RotateCcw className="h-3.5 w-3.5" />
            <span>Reset Filters</span>
          </Button>
          <span className="text-xs text-muted-foreground">
            {totalCount} {totalCount === 1 ? 'program' : 'programs'}
          </span>
        </div>
      </div>
    </section>
  )
}
