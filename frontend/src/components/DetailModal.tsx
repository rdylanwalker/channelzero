import { Bookmark, Check, ExternalLink, EyeOff, Film, Star, Tv } from 'lucide-react'
import type { ProgramDto, ProviderMeta } from '../types'
import { Button } from './ui/Button'
import { Dialog } from './ui/Dialog'

interface DetailModalProps {
  program: ProgramDto | null
  providerMetaMap: Map<string, ProviderMeta>
  onClose: () => void
  onLaunch: (program: ProgramDto) => void
  onToggleWatched: (program: ProgramDto) => void
  onToggleWatchlist: (program: ProgramDto) => void
  onNotInterested: (program: ProgramDto) => void
  onHideGenre: (genre: string) => void
}

export function DetailModal({
  program,
  providerMetaMap,
  onClose,
  onLaunch,
  onToggleWatched,
  onToggleWatchlist,
  onNotInterested,
  onHideGenre,
}: DetailModalProps) {
  if (!program) return null

  return (
    <Dialog open={!!program} onClose={onClose} title={program.title}>
      <div className="space-y-5">
        <div className="flex flex-col gap-4 sm:flex-row">
          {program.poster_url ? (
            <img
              src={program.poster_url}
              alt={program.title}
              className="h-64 w-44 flex-shrink-0 rounded-lg object-cover shadow-md"
            />
          ) : (
            <div className="flex h-64 w-44 flex-shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              {program.media_type === 'movie' ? (
                <Film className="h-12 w-12 opacity-50" />
              ) : (
                <Tv className="h-12 w-12 opacity-50" />
              )}
            </div>
          )}

          <div className="flex flex-1 flex-col justify-between space-y-3">
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <span className="rounded bg-muted px-2 py-0.5 text-xs font-semibold capitalize text-foreground">
                  {program.media_type}
                </span>
                <span className="text-xs text-muted-foreground">
                  Released {program.release_date}
                </span>
                {program.release_year && (
                  <span className="text-xs text-muted-foreground">({program.release_year})</span>
                )}
                <span className="rounded bg-accent/20 px-2 py-0.5 text-xs font-medium text-blue-300">
                  {program.release_kind === 'new_title' ? 'New Series / Movie' : 'New Episode'}
                </span>
              </div>

              {program.episode_info && (
                <p className="mt-2 text-sm font-medium text-accent">{program.episode_info}</p>
              )}

              {/* Source Ratings Breakdown */}
              {program.ratings.length > 0 && (
                <div className="mt-3 flex flex-wrap items-center gap-2">
                  {program.ratings.map((r) => (
                    <div
                      key={r.source}
                      className="flex items-center gap-1 rounded-md border border-border bg-muted/50 px-2.5 py-1 text-xs"
                    >
                      <Star className="h-3 w-3 fill-amber-400 text-amber-400" />
                      <span className="font-semibold text-foreground">{r.source}:</span>
                      <span className="text-muted-foreground">{r.score}</span>
                    </div>
                  ))}
                </div>
              )}

              {/* Genres with quick hide button */}
              {program.genres.length > 0 && (
                <div className="mt-3 flex flex-wrap items-center gap-1.5">
                  <span className="text-xs text-muted-foreground">Genres:</span>
                  {program.genres.map((g) => (
                    <span
                      key={g}
                      className="inline-flex items-center gap-1 rounded-full border border-border bg-muted/40 pl-2 pr-1 py-0.5 text-xs text-foreground"
                    >
                      <span>{g}</span>
                      <button
                        type="button"
                        onClick={() => onHideGenre(g)}
                        className="rounded-full p-0.5 text-muted-foreground hover:bg-muted hover:text-red-400"
                        title={`Hide all "${g}" titles from feed`}
                      >
                        <EyeOff className="h-3 w-3" />
                      </button>
                    </span>
                  ))}
                </div>
              )}
            </div>

            {/* Quick Actions */}
            <div className="flex flex-wrap items-center gap-2 border-t border-border pt-3">
              <Button
                variant={program.watched ? 'default' : 'outline'}
                size="sm"
                onClick={() => onToggleWatched(program)}
              >
                <Check className="h-4 w-4" />
                <span>{program.watched ? 'Watched' : 'Mark Watched'}</span>
              </Button>

              <Button
                variant={program.watchlist ? 'secondary' : 'outline'}
                size="sm"
                onClick={() => onToggleWatchlist(program)}
              >
                <Bookmark className="h-4 w-4" />
                <span>{program.watchlist ? 'In Watchlist' : 'Save to Watchlist'}</span>
              </Button>

              <Button
                variant="destructive"
                size="sm"
                onClick={() => {
                  onNotInterested(program)
                  onClose()
                }}
              >
                <EyeOff className="h-4 w-4" />
                <span>Not Interested</span>
              </Button>
            </div>
          </div>
        </div>

        {/* Synopsis */}
        {program.synopsis && (
          <div className="space-y-1 rounded-lg border border-border bg-muted/30 p-3">
            <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Synopsis
            </h4>
            <p className="text-sm leading-relaxed text-foreground">{program.synopsis}</p>
          </div>
        )}

        {/* Available Streaming Providers (Deep Links) */}
        <div className="space-y-2">
          <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Where to Watch
          </h4>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            {program.providers.map((prov) => {
              const meta = providerMetaMap.get(prov.id)
              return (
                <a
                  key={prov.id}
                  href={prov.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={() => onLaunch(program)}
                  className="flex items-center justify-between rounded-lg border border-border bg-muted/40 p-3 transition hover:border-accent hover:bg-muted"
                >
                  <div className="flex items-center gap-2.5">
                    <span
                      className="h-3 w-3 rounded-full"
                      style={{ backgroundColor: meta?.color ?? '#3b82f6' }}
                    />
                    <div>
                      <div className="font-medium text-foreground">{prov.name}</div>
                      <div className="text-[11px] text-muted-foreground">
                        {prov.is_direct ? 'Direct link' : 'Provider search'}
                      </div>
                    </div>
                  </div>
                  <ExternalLink className="h-4 w-4 text-muted-foreground" />
                </a>
              )
            })}
          </div>
        </div>

        {/* Metadata Source Attribution */}
        <div className="flex items-center justify-between border-t border-border pt-2 text-[11px] text-muted-foreground">
          <span>Metadata sources: {program.sources.join(', ') || 'TVmaze'}</span>
          {program.imdb_id && (
            <a
              href={`https://www.imdb.com/title/${program.imdb_id}/`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-accent hover:underline"
            >
              IMDb Page
            </a>
          )}
        </div>
      </div>
    </Dialog>
  )
}
