import { Activity, RefreshCw, Settings, Tv, User } from 'lucide-react'
import type { Profile, PublicConfig, SyncStatusResponse } from '../types'
import { Button } from './ui/Button'
import { Select } from './ui/Select'

interface HeaderProps {
  profiles: Profile[]
  activeProfileId: string
  onSelectProfile: (id: string) => void
  config: PublicConfig | null
  syncStatus: SyncStatusResponse | null
  onRefresh: () => void
  onOpenSyncStatus: () => void
  onOpenProfileSettings: () => void
}

export function Header({
  profiles,
  activeProfileId,
  onSelectProfile,
  config,
  syncStatus,
  onRefresh,
  onOpenSyncStatus,
  onOpenProfileSettings,
}: HeaderProps) {
  const isSyncing = syncStatus?.is_syncing ?? false
  const hasError = syncStatus?.sources.some((s) => s.enabled && s.last_status === 'error')
  const activeProfile = profiles.find((p) => p.id === activeProfileId)
  const effectiveRegion = activeProfile?.region ?? config?.region
  const effectiveDays = activeProfile?.new_release_days ?? config?.new_release_days

  return (
    <header className="sticky top-0 z-30 border-b border-border bg-background/90 backdrop-blur-md">
      <div className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-3 px-4 py-3">
        <div className="flex items-center gap-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-accent text-white shadow">
            <Tv className="h-5 w-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-lg font-bold tracking-tight text-foreground">ChannelZero</h1>
              {effectiveRegion && effectiveDays !== undefined && (
                <span className="rounded bg-muted px-1.5 py-0.5 text-[11px] font-medium text-muted-foreground">
                  {effectiveRegion} · {effectiveDays}d
                </span>
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              New releases across your streaming services
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-1.5 rounded-md border border-border bg-muted/50 px-2 py-1">
            <User className="h-4 w-4 text-muted-foreground" />
            <Select
              value={activeProfileId}
              onChange={(e) => onSelectProfile(e.target.value)}
              className="h-7 border-0 bg-transparent px-1 text-xs focus-visible:ring-0"
              aria-label="Active profile"
            >
              {profiles.map((p) => (
                <option key={p.id} value={p.id} className="bg-card text-foreground">
                  {p.name}
                </option>
              ))}
            </Select>
          </div>

          <Button
            variant="outline"
            size="sm"
            onClick={onOpenProfileSettings}
            title="Profiles & Hidden Titles/Genres"
          >
            <Settings className="h-4 w-4" />
            <span className="hidden sm:inline">Preferences</span>
          </Button>

          <Button variant="outline" size="sm" onClick={onOpenSyncStatus} title="API & Sync Status">
            <Activity className={`h-4 w-4 ${hasError ? 'text-amber-400' : 'text-emerald-400'}`} />
            <span className="hidden sm:inline">Sources</span>
          </Button>

          <Button
            variant="default"
            size="sm"
            onClick={onRefresh}
            disabled={isSyncing}
            title="Refresh metadata now"
          >
            <RefreshCw className={`h-4 w-4 ${isSyncing ? 'animate-spin' : ''}`} />
            <span>{isSyncing ? 'Syncing...' : 'Refresh'}</span>
          </Button>
        </div>
      </div>
    </header>
  )
}
