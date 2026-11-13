import { ArrowDownUp, Search } from 'lucide-react'
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
  genre: string
  onGenreChange: (v: string) => void
  availableGenres: string[]
  status: string
  onStatusChange: (v: string) => void
  minRating: string
  onMinRatingChange: (v: string) => void
  sort: string
  onSortChange: (v: string) => void
  order: 'asc' | 'desc'
  onOrderToggle: () => void
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
  genre,
  onGenreChange,
  availableGenres,
  status,
  onStatusChange,
  minRating,
  onMinRatingChange,
  sort,
  onSortChange,
  order,
  onOrderToggle,
  totalCount,
}: FilterBarProps) {
  return (
    <section className="space-y-3 rounded-xl border border-border bg-card/70 p-4">
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

          <Select value={genre} onChange={(e) => onGenreChange(e.target.value)}>
            <option value="all">All Genres</option>
            {availableGenres.map((g) => (
              <option key={g} value={g}>
                {g}
              </option>
            ))}
          </Select>

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

        <span className="text-xs text-muted-foreground">
          {totalCount} {totalCount === 1 ? 'program' : 'programs'}
        </span>
      </div>
    </section>
  )
}
