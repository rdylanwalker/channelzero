import { Bookmark, Check, Clock, Eye, EyeOff, Film, Play, Star, Tv } from 'lucide-react'
import type { ProgramDto, ProviderMeta } from '../types'
import { Badge } from './ui/Badge'

interface ProgramCardProps {
  program: ProgramDto
  providerMetaMap: Map<string, ProviderMeta>
  onLaunch: (program: ProgramDto) => void
  onOpenDetail: (program: ProgramDto) => void
  onToggleWatched: (program: ProgramDto) => void
  onToggleWatchlist: (program: ProgramDto) => void
  onNotInterested: (program: ProgramDto) => void
}

export function ProgramCard({
  program,
  providerMetaMap,
  onLaunch,
  onOpenDetail,
  onToggleWatched,
  onToggleWatchlist,
  onNotInterested,
}: ProgramCardProps) {
  return (
    <article
      className={`group flex flex-col overflow-hidden rounded-xl border border-border bg-card transition hover:border-accent/60 hover:shadow-lg ${
        program.watched ? 'opacity-75' : ''
      }`}
    >
      {/* Poster + Play Overlay (click launches primary streaming service & marks seen) */}
      <a
        href={program.primary_url}
        target="_blank"
        rel="noopener noreferrer"
        onClick={() => onLaunch(program)}
        className="relative block aspect-[2/3] w-full overflow-hidden bg-muted"
        title={`Watch ${program.title} on ${program.providers[0]?.name ?? 'streaming service'}`}
      >
        {program.poster_url ? (
          <img
            src={program.poster_url}
            alt={program.title}
            loading="lazy"
            className="h-full w-full object-cover transition duration-300 group-hover:scale-105"
          />
        ) : (
          <div className="flex h-full w-full flex-col items-center justify-center gap-2 p-4 text-center text-muted-foreground">
            {program.media_type === 'movie' ? (
              <Film className="h-10 w-10 opacity-50" />
            ) : (
              <Tv className="h-10 w-10 opacity-50" />
            )}
            <span className="text-xs font-medium">{program.title}</span>
          </div>
        )}

        <div className="absolute inset-0 flex items-center justify-center bg-black/45 opacity-0 transition-opacity group-hover:opacity-100">
          <div className="flex h-12 w-12 items-center justify-center rounded-full bg-accent text-white shadow-lg">
            <Play className="ml-0.5 h-6 w-6 fill-current" />
          </div>
        </div>

        {/* Top-left status badges */}
        <div className="absolute left-2 top-2 flex flex-col gap-1">
          {program.outside_window && (
            <Badge
              variant="outline"
              className="border-amber-400/50 bg-black/75 text-amber-200 backdrop-blur-md"
            >
              <Clock className="h-3 w-3" /> Older than window
            </Badge>
          )}
          {program.watched && (
            <Badge variant="success" className="backdrop-blur-md">
              <Check className="h-3 w-3" /> Watched
            </Badge>
          )}
          {!program.watched && program.seen && (
            <Badge variant="default" className="backdrop-blur-md">
              <Eye className="h-3 w-3" /> Seen
            </Badge>
          )}
          {program.watchlist && (
            <Badge variant="warning" className="backdrop-blur-md">
              <Bookmark className="h-3 w-3" /> Saved
            </Badge>
          )}
        </div>

        {/* Top-right rating */}
        {program.rating !== null && (
          <div className="absolute right-2 top-2 flex items-center gap-1 rounded-full bg-black/75 px-2 py-0.5 text-xs font-semibold text-amber-300 backdrop-blur-md">
            <Star className="h-3 w-3 fill-amber-300 text-amber-300" />
            <span>{program.rating.toFixed(1)}</span>
          </div>
        )}
      </a>

      {/* Card info area (click opens detail modal) */}
      <div className="flex flex-1 flex-col justify-between p-3">
        <div>
          <button
            type="button"
            onClick={() => onOpenDetail(program)}
            className="w-full text-left focus:outline-none"
          >
            <h3 className="line-clamp-1 text-sm font-semibold text-foreground hover:text-accent">
              {program.title}
            </h3>
            <div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
              <span>{program.release_date}</span>
              <span>·</span>
              <span className="capitalize">{program.media_type}</span>
            </div>
            {program.episode_info && (
              <p className="mt-1 line-clamp-1 text-[11px] font-medium text-blue-300">
                {program.episode_info}
              </p>
            )}
          </button>

          {/* Provider Badges */}
          <div className="mt-2.5 flex flex-wrap gap-1.5">
            {program.providers.map((prov) => {
              const meta = providerMetaMap.get(prov.id)
              return (
                <a
                  key={prov.id}
                  href={prov.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={() => onLaunch(program)}
                  className="inline-flex items-center gap-1 rounded-md border border-border bg-muted/60 px-2 py-0.5 text-[11px] font-medium text-foreground transition hover:border-accent hover:bg-muted"
                  title={`Open on ${prov.name}${prov.is_direct ? ' (direct link)' : ' (search)'}`}
                >
                  <span
                    className="h-2 w-2 rounded-full"
                    style={{ backgroundColor: meta?.color ?? '#64748b' }}
                  />
                  {prov.name}
                </a>
              )
            })}
          </div>
        </div>

        {/* Footer action buttons */}
        <div className="mt-3 flex items-center justify-between border-t border-border/60 pt-2">
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={() => onToggleWatched(program)}
              className={`rounded-md p-1.5 text-xs transition ${
                program.watched
                  ? 'bg-emerald-500/20 text-emerald-300'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground'
              }`}
              title={program.watched ? 'Unmark watched' : 'Mark as watched'}
            >
              <Check className="h-4 w-4" />
            </button>

            <button
              type="button"
              onClick={() => onToggleWatchlist(program)}
              className={`rounded-md p-1.5 text-xs transition ${
                program.watchlist
                  ? 'bg-amber-500/20 text-amber-300'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground'
              }`}
              title={program.watchlist ? 'Remove from watchlist' : 'Save to watchlist'}
            >
              <Bookmark className="h-4 w-4" />
            </button>
          </div>

          <button
            type="button"
            onClick={() => onNotInterested(program)}
            className="rounded-md p-1.5 text-xs text-muted-foreground transition hover:bg-red-500/15 hover:text-red-300"
            title="Not interested (hide title)"
          >
            <EyeOff className="h-4 w-4" />
          </button>
        </div>
      </div>
    </article>
  )
}
