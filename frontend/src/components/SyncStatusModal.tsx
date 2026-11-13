import { AlertCircle, CheckCircle2, Clock, KeyRound, RefreshCw, XCircle } from 'lucide-react'
import type { SyncStatusResponse } from '../types'
import { Badge } from './ui/Badge'
import { Button } from './ui/Button'
import { Dialog } from './ui/Dialog'

interface SyncStatusModalProps {
  open: boolean
  onClose: () => void
  status: SyncStatusResponse | null
  onTriggerRefresh: () => void
}

export function SyncStatusModal({ open, onClose, status, onTriggerRefresh }: SyncStatusModalProps) {
  if (!status) return null

  return (
    <Dialog open={open} onClose={onClose} title="Metadata Sources & Sync Status">
      <div className="space-y-5">
        {/* Top Summary Card */}
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div className="rounded-lg border border-border bg-muted/30 p-3">
            <span className="text-[11px] font-medium text-muted-foreground">Region</span>
            <div className="mt-1 text-lg font-bold text-foreground">{status.region}</div>
          </div>
          <div className="rounded-lg border border-border bg-muted/30 p-3">
            <span className="text-[11px] font-medium text-muted-foreground">Total Titles</span>
            <div className="mt-1 text-lg font-bold text-foreground">{status.total_programs}</div>
          </div>
          <div className="rounded-lg border border-border bg-muted/30 p-3">
            <span className="text-[11px] font-medium text-muted-foreground">Lookback</span>
            <div className="mt-1 text-lg font-bold text-foreground">
              {status.new_release_days} days
            </div>
          </div>
          <div className="rounded-lg border border-border bg-muted/30 p-3">
            <span className="text-[11px] font-medium text-muted-foreground">Auto-Refresh</span>
            <div className="mt-1 text-lg font-bold text-foreground">
              Every {status.refresh_interval_hours}h
            </div>
          </div>
        </div>

        {/* Sync Controls */}
        <div className="flex items-center justify-between rounded-lg border border-border bg-card p-3">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Clock className="h-4 w-4" />
            <span>
              Last sync:{' '}
              <strong className="text-foreground">
                {status.last_sync_at ? new Date(status.last_sync_at).toLocaleString() : 'Never'}
              </strong>
            </span>
          </div>

          <Button
            variant="default"
            size="sm"
            onClick={onTriggerRefresh}
            disabled={status.is_syncing}
          >
            <RefreshCw className={`h-4 w-4 ${status.is_syncing ? 'animate-spin' : ''}`} />
            <span>{status.is_syncing ? 'Syncing...' : 'Sync Now'}</span>
          </Button>
        </div>

        {/* Upstream Sources List */}
        <div className="space-y-2">
          <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Configured Metadata Sources
          </h4>
          <div className="space-y-2">
            {status.sources.map((src) => {
              const isOk = src.enabled && src.configured && src.last_status === 'ok'
              const isMissingKey = src.enabled && !src.configured
              const isError = src.enabled && src.last_status === 'error'
              const isDisabled = !src.enabled

              return (
                <div
                  key={src.name}
                  className="flex flex-col gap-2 rounded-lg border border-border bg-muted/30 p-3 sm:flex-row sm:items-center sm:justify-between"
                >
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-foreground">{src.name}</span>
                      <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                        {src.tier}
                      </span>
                      {isOk && (
                        <Badge variant="success">
                          <CheckCircle2 className="h-3 w-3" /> Active
                        </Badge>
                      )}
                      {isMissingKey && (
                        <Badge variant="warning">
                          <KeyRound className="h-3 w-3" /> Key Required
                        </Badge>
                      )}
                      {isError && (
                        <Badge variant="destructive">
                          <AlertCircle className="h-3 w-3" /> Sync Error
                        </Badge>
                      )}
                      {isDisabled && (
                        <Badge variant="secondary">
                          <XCircle className="h-3 w-3" /> Disabled
                        </Badge>
                      )}
                    </div>
                    {src.last_error && <p className="text-xs text-red-400">{src.last_error}</p>}
                  </div>

                  <div className="text-right text-xs text-muted-foreground">
                    <div>{src.items_fetched} items processed</div>
                    {src.last_sync_at && (
                      <div className="text-[11px]">
                        {new Date(src.last_sync_at).toLocaleTimeString()}
                      </div>
                    )}
                  </div>
                </div>
              )
            })}
          </div>
        </div>

        {/* Guidance note */}
        <p className="rounded-md border border-border/60 bg-muted/20 p-2.5 text-xs text-muted-foreground">
          Tip: <strong>TVmaze</strong> works out of the box with no API key. To unlock movie
          releases, add a free <strong>TMDB</strong> key (
          <code className="text-foreground">TMDB_API_KEY</code>). Add <strong>OMDb</strong> (
          <code className="text-foreground">OMDB_API_KEY</code>) to enrich titles with Rotten
          Tomatoes and IMDb scores.
        </p>
      </div>
    </Dialog>
  )
}
