import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  createProfile,
  deleteProfile,
  getHiddenState,
  getPublicConfig,
  getSyncStatus,
  hideGenre,
  listGenres,
  listProfiles,
  listPrograms,
  triggerSyncRefresh,
  unhideGenre,
  updateInteraction,
  updateProfileSettings,
} from './api/client'
import { DetailModal } from './components/DetailModal'
import { FilterBar } from './components/FilterBar'
import { Header } from './components/Header'
import { ProfileSettingsModal } from './components/ProfileSettingsModal'
import { ProgramCard } from './components/ProgramCard'
import { SyncStatusModal } from './components/SyncStatusModal'
import type {
  Profile,
  ProfileHiddenState,
  ProgramDto,
  ProviderMeta,
  PublicConfig,
  SyncStatusResponse,
} from './types'

export function App() {
  const [config, setConfig] = useState<PublicConfig | null>(null)
  const [profiles, setProfiles] = useState<Profile[]>([])
  const [activeProfileId, setActiveProfileId] = useState<string>('default')
  const [syncStatus, setSyncStatus] = useState<SyncStatusResponse | null>(null)
  const [programs, setPrograms] = useState<ProgramDto[]>([])
  const [visibleCount, setVisibleCount] = useState(24)
  const loadMoreRef = useRef<HTMLDivElement | null>(null)
  const didMountFiltersRef = useRef(false)
  const [availableGenres, setAvailableGenres] = useState<string[]>([])
  const [hiddenState, setHiddenState] = useState<ProfileHiddenState | null>(null)
  const [loading, setLoading] = useState(true)

  // Filters state
  const [q, setQ] = useState('')
  const [selectedProvider, setSelectedProvider] = useState('all')
  const [mediaType, setMediaType] = useState('all')
  const [releaseKind, setReleaseKind] = useState('all')
  const [genre, setGenre] = useState('all')
  const [status, setStatus] = useState('all')
  const [minRating, setMinRating] = useState('all')
  const [sort, setSort] = useState('release_date')
  const [order, setOrder] = useState<'asc' | 'desc'>('desc')

  // Modals state
  const [selectedProgram, setSelectedProgram] = useState<ProgramDto | null>(null)
  const [syncModalOpen, setSyncModalOpen] = useState(false)
  const [profileModalOpen, setProfileModalOpen] = useState(false)

  // Provider map lookup
  const providerMetaMap = useMemo(() => {
    const map = new Map<string, ProviderMeta>()
    if (config) {
      for (const p of config.providers) {
        map.set(p.id, p)
      }
    }
    return map
  }, [config])

  // Initial bootstrap
  useEffect(() => {
    async function init() {
      try {
        const [cfg, profs, sync] = await Promise.all([
          getPublicConfig(),
          listProfiles(),
          getSyncStatus(),
        ])
        setConfig(cfg)
        setProfiles(profs)
        setSyncStatus(sync)

        const savedProfile = localStorage.getItem('channelzero_profile')
        if (savedProfile && profs.some((p) => p.id === savedProfile)) {
          setActiveProfileId(savedProfile)
        } else if (profs.length > 0) {
          setActiveProfileId(profs[0].id)
        }
      } catch (err) {
        console.error('Initialization error:', err)
      }
    }
    init()
  }, [])

  // Fetch programs, available genres & hidden state for active profile
  const refreshPrograms = useCallback(async () => {
    if (!activeProfileId) return
    try {
      const [progs, hidden, genres] = await Promise.all([
        listPrograms({
          profileId: activeProfileId,
          q,
          provider: selectedProvider,
          mediaType,
          releaseKind,
          genre,
          status,
          minRating,
          sort,
          order,
        }),
        getHiddenState(activeProfileId),
        listGenres(activeProfileId),
      ])
      setPrograms(progs)
      setVisibleCount(24)
      setHiddenState(hidden)
      setAvailableGenres(genres)
    } catch (err) {
      console.error('Failed to load programs:', err)
    } finally {
      setLoading(false)
    }
  }, [
    activeProfileId,
    q,
    selectedProvider,
    mediaType,
    releaseKind,
    genre,
    status,
    minRating,
    sort,
    order,
  ])

  // Poll sync status when syncing
  useEffect(() => {
    if (!syncStatus?.is_syncing) return
    const interval = setInterval(async () => {
      try {
        const nextStatus = await getSyncStatus(activeProfileId)
        setSyncStatus(nextStatus)
        if (!nextStatus.is_syncing) {
          refreshPrograms()
        }
      } catch (err) {
        console.error('Sync status poll error:', err)
      }
    }, 2000)
    return () => clearInterval(interval)
  }, [syncStatus?.is_syncing, activeProfileId, refreshPrograms])

  useEffect(() => {
    void (async () => {
      await refreshPrograms()
    })()
  }, [refreshPrograms])

  // Reset pagination and scroll back to top whenever a filter, sort, or profile changes
  useEffect(() => {
    if (!didMountFiltersRef.current) {
      didMountFiltersRef.current = true
      return
    }
    setVisibleCount(24)
    if (typeof window.scrollTo === 'function') {
      window.scrollTo({ top: 0, behavior: 'smooth' })
    }
  }, [
    activeProfileId,
    q,
    selectedProvider,
    mediaType,
    releaseKind,
    genre,
    status,
    minRating,
    sort,
    order,
  ])

  const handleResetFilters = () => {
    setQ('')
    setSelectedProvider('all')
    setMediaType('all')
    setReleaseKind('all')
    setGenre('all')
    setStatus('all')
    setMinRating('all')
    setSort('release_date')
    setOrder('desc')
  }

  const handleSelectProfile = (id: string) => {
    setActiveProfileId(id)
    localStorage.setItem('channelzero_profile', id)
  }

  const handleCreateProfile = async (name: string) => {
    const created = await createProfile(name)
    const list = await listProfiles()
    setProfiles(list)
    handleSelectProfile(created.id)
  }

  const handleDeleteProfile = async (id: string) => {
    await deleteProfile(id)
    const list = await listProfiles()
    setProfiles(list)
    if (activeProfileId === id) {
      handleSelectProfile(list[0]?.id ?? 'default')
    }
  }

  const handleUpdateProfileSettings = async (
    id: string,
    patch: { region: string | null; new_release_days: number | null },
  ) => {
    const updated = await updateProfileSettings(id, patch)
    setProfiles((prev) => prev.map((p) => (p.id === id ? updated : p)))
    refreshPrograms()
  }

  const handleManualSync = async () => {
    await triggerSyncRefresh(activeProfileId)
    const nextStatus = await getSyncStatus(activeProfileId)
    setSyncStatus(nextStatus)
  }

  // Interactivity handlers
  const handleLaunch = async (program: ProgramDto) => {
    await updateInteraction(activeProfileId, program.id, { seen: true })
    setPrograms((prev) => prev.map((p) => (p.id === program.id ? { ...p, seen: true } : p)))
  }

  const handleToggleWatched = async (program: ProgramDto) => {
    const nextWatched = !program.watched
    await updateInteraction(activeProfileId, program.id, {
      watched: nextWatched,
      seen: nextWatched ? true : program.seen,
    })
    setPrograms((prev) =>
      prev.map((p) =>
        p.id === program.id ? { ...p, watched: nextWatched, seen: nextWatched ? true : p.seen } : p,
      ),
    )
    if (selectedProgram?.id === program.id) {
      setSelectedProgram((prev) =>
        prev ? { ...prev, watched: nextWatched, seen: nextWatched ? true : prev.seen } : null,
      )
    }
  }

  const handleToggleWatchlist = async (program: ProgramDto) => {
    const nextWatchlist = !program.watchlist
    await updateInteraction(activeProfileId, program.id, { watchlist: nextWatchlist })
    setPrograms((prev) =>
      prev.map((p) => (p.id === program.id ? { ...p, watchlist: nextWatchlist } : p)),
    )
    if (selectedProgram?.id === program.id) {
      setSelectedProgram((prev) => (prev ? { ...prev, watchlist: nextWatchlist } : null))
    }
  }

  const handleNotInterested = async (program: ProgramDto) => {
    await updateInteraction(activeProfileId, program.id, { not_interested: true })
    setPrograms((prev) => prev.filter((p) => p.id !== program.id))
    const hidden = await getHiddenState(activeProfileId)
    setHiddenState(hidden)
  }

  const handleUnhideTitle = async (programId: string) => {
    await updateInteraction(activeProfileId, programId, { not_interested: false })
    refreshPrograms()
  }

  const handleHideGenre = async (genreName: string) => {
    await hideGenre(activeProfileId, genreName)
    refreshPrograms()
  }

  const handleUnhideGenre = async (genreName: string) => {
    await unhideGenre(activeProfileId, genreName)
    refreshPrograms()
  }

  const visiblePrograms = useMemo(() => programs.slice(0, visibleCount), [programs, visibleCount])
  const hasMorePrograms = visibleCount < programs.length

  useEffect(() => {
    if (!hasMorePrograms || typeof IntersectionObserver === 'undefined') return
    const el = loadMoreRef.current
    if (!el) return

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          setVisibleCount((prev) => Math.min(prev + 24, programs.length))
        }
      },
      { rootMargin: '200px' },
    )
    observer.observe(el)
    return () => observer.disconnect()
  }, [hasMorePrograms, programs.length])

  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="sticky top-0 z-30 bg-background/90 backdrop-blur-md">
        <Header
          profiles={profiles}
          activeProfileId={activeProfileId}
          onSelectProfile={handleSelectProfile}
          config={config}
          syncStatus={syncStatus}
          onRefresh={handleManualSync}
          onOpenSyncStatus={() => setSyncModalOpen(true)}
          onOpenProfileSettings={() => setProfileModalOpen(true)}
        />

        <div className="mx-auto max-w-7xl px-4 py-3">
          <FilterBar
            q={q}
            onQChange={setQ}
            providers={config?.providers ?? []}
            selectedProvider={selectedProvider}
            onProviderChange={setSelectedProvider}
            mediaType={mediaType}
            onMediaTypeChange={setMediaType}
            releaseKind={releaseKind}
            onReleaseKindChange={setReleaseKind}
            genre={genre}
            onGenreChange={setGenre}
            availableGenres={availableGenres}
            status={status}
            onStatusChange={setStatus}
            minRating={minRating}
            onMinRatingChange={setMinRating}
            sort={sort}
            onSortChange={setSort}
            order={order}
            onOrderToggle={() => setOrder((prev) => (prev === 'desc' ? 'asc' : 'desc'))}
            onResetFilters={handleResetFilters}
            totalCount={programs.length}
          />
        </div>
      </div>

      <main className="mx-auto max-w-7xl space-y-6 px-4 pb-6 pt-3">
        {loading ? (
          <div className="flex h-64 items-center justify-center">
            <div className="h-8 w-8 animate-spin rounded-full border-2 border-accent border-t-transparent" />
          </div>
        ) : programs.length === 0 ? (
          <div className="rounded-xl border border-dashed border-border p-12 text-center">
            <h3 className="text-base font-semibold text-foreground">No new releases found</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Try broadening your filters or click Refresh to sync the latest streaming releases.
            </p>
          </div>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
              {visiblePrograms.map((p) => (
                <ProgramCard
                  key={p.id}
                  program={p}
                  providerMetaMap={providerMetaMap}
                  onLaunch={handleLaunch}
                  onOpenDetail={setSelectedProgram}
                  onToggleWatched={handleToggleWatched}
                  onToggleWatchlist={handleToggleWatchlist}
                  onNotInterested={handleNotInterested}
                />
              ))}
            </div>

            {hasMorePrograms && (
              <div ref={loadMoreRef} className="flex flex-col items-center gap-2 py-4">
                <button
                  type="button"
                  onClick={() => setVisibleCount((prev) => Math.min(prev + 24, programs.length))}
                  className="rounded-md border border-border bg-muted/50 px-4 py-2 text-xs font-medium text-foreground transition hover:bg-muted"
                >
                  Load more ({programs.length - visiblePrograms.length} remaining)
                </button>
              </div>
            )}
          </>
        )}
      </main>

      <DetailModal
        program={selectedProgram}
        providerMetaMap={providerMetaMap}
        onClose={() => setSelectedProgram(null)}
        onLaunch={handleLaunch}
        onToggleWatched={handleToggleWatched}
        onToggleWatchlist={handleToggleWatchlist}
        onNotInterested={handleNotInterested}
        onHideGenre={handleHideGenre}
      />

      <SyncStatusModal
        open={syncModalOpen}
        onClose={() => setSyncModalOpen(false)}
        status={syncStatus}
        onTriggerRefresh={handleManualSync}
      />

      <ProfileSettingsModal
        open={profileModalOpen}
        onClose={() => setProfileModalOpen(false)}
        profiles={profiles}
        activeProfileId={activeProfileId}
        config={config}
        onSelectProfile={handleSelectProfile}
        onCreateProfile={handleCreateProfile}
        onDeleteProfile={handleDeleteProfile}
        onUpdateProfileSettings={handleUpdateProfileSettings}
        onTriggerSync={handleManualSync}
        hiddenState={hiddenState}
        onUnhideTitle={handleUnhideTitle}
        onUnhideGenre={handleUnhideGenre}
      />
    </div>
  )
}
