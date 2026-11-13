import { useEffect, useRef, useState } from 'react'
import { Activity, Check, ChevronDown, RefreshCw, Settings, User } from 'lucide-react'
import type { Profile, PublicConfig, SyncStatusResponse } from '../types'
import { Button } from './ui/Button'

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
  const [profileMenuOpen, setProfileMenuOpen] = useState(false)
  const profileMenuRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    if (!profileMenuOpen) return
    const handleMouseDown = (e: MouseEvent) => {
      if (profileMenuRef.current && !profileMenuRef.current.contains(e.target as Node)) {
        setProfileMenuOpen(false)
      }
    }
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setProfileMenuOpen(false)
      }
    }
    document.addEventListener('mousedown', handleMouseDown)
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('mousedown', handleMouseDown)
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [profileMenuOpen])

  const isSyncing = syncStatus?.is_syncing ?? false
  const hasError = syncStatus?.sources.some((s) => s.enabled && s.last_status === 'error')
  const activeProfile = profiles.find((p) => p.id === activeProfileId)
  const effectiveRegion = activeProfile?.region ?? config?.region
  const effectiveDays = activeProfile?.new_release_days ?? config?.new_release_days
  const lastSyncLabel = syncStatus
    ? syncStatus.last_sync_at
      ? new Date(syncStatus.last_sync_at).toLocaleString()
      : 'Never'
    : null

  return (
    <header className="border-b border-border bg-background/90 backdrop-blur-md">
      <div className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-3 px-4 py-3">
        <div className="flex items-center gap-3">
          <img src="/favicon.svg" alt="ChannelZero" className="h-9 w-9 rounded-lg shadow" />
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
              {lastSyncLabel !== null && (
                <span className="ml-1.5" data-testid="last-refreshed">
                  · Last refreshed: <span className="text-foreground/80">{lastSyncLabel}</span>
                </span>
              )}
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <div className="relative" ref={profileMenuRef}>
            <button
              type="button"
              onClick={() => setProfileMenuOpen((prev) => !prev)}
              aria-label="Active profile"
              aria-haspopup="listbox"
              aria-expanded={profileMenuOpen}
              className="flex h-8 items-center gap-1.5 rounded-md border border-border bg-muted/50 px-2.5 text-xs font-medium text-foreground shadow-sm transition hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              <User className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="max-w-[120px] truncate">{activeProfile?.name ?? 'Default'}</span>
              <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
            </button>

            {profileMenuOpen && (
              <div
                role="listbox"
                aria-label="Profiles"
                className="absolute right-0 top-full z-40 mt-1 min-w-[160px] rounded-lg border border-border bg-card p-1 shadow-xl"
              >
                {profiles.map((p) => {
                  const active = p.id === activeProfileId
                  return (
                    <button
                      key={p.id}
                      type="button"
                      role="option"
                      aria-selected={active}
                      onClick={() => {
                        onSelectProfile(p.id)
                        setProfileMenuOpen(false)
                      }}
                      className={`flex w-full items-center justify-between gap-2 rounded-md px-2.5 py-1.5 text-left text-xs transition ${
                        active
                          ? 'bg-accent/15 font-medium text-foreground'
                          : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                      }`}
                    >
                      <span className="truncate">{p.name}</span>
                      {active && <Check className="h-3.5 w-3.5 shrink-0 text-accent" />}
                    </button>
                  )
                })}
              </div>
            )}
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
