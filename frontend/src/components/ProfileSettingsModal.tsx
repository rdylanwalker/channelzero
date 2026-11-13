import { useState } from 'react'
import { Plus, RefreshCw, Trash2, Undo } from 'lucide-react'
import type { Profile, ProfileHiddenState, PublicConfig } from '../types'
import { Button } from './ui/Button'
import { Dialog } from './ui/Dialog'
import { Input } from './ui/Input'

interface ProfileSettingsModalProps {
  open: boolean
  onClose: () => void
  profiles: Profile[]
  activeProfileId: string
  config: PublicConfig | null
  onSelectProfile: (id: string) => void
  onCreateProfile: (name: string) => Promise<void>
  onDeleteProfile: (id: string) => Promise<void>
  onUpdateProfileSettings: (
    id: string,
    patch: { region: string | null; new_release_days: number | null },
  ) => Promise<void>
  onTriggerSync: () => Promise<void>
  hiddenState: ProfileHiddenState | null
  onUnhideTitle: (programId: string) => Promise<void>
  onUnhideGenre: (genre: string) => Promise<void>
}

export function ProfileSettingsModal({
  open,
  onClose,
  profiles,
  activeProfileId,
  config,
  onSelectProfile,
  onCreateProfile,
  onDeleteProfile,
  onUpdateProfileSettings,
  onTriggerSync,
  hiddenState,
  onUnhideTitle,
  onUnhideGenre,
}: ProfileSettingsModalProps) {
  const [newProfileName, setNewProfileName] = useState('')
  const [creating, setCreating] = useState(false)

  const activeProfile = profiles.find((p) => p.id === activeProfileId)

  const [prevSyncKey, setPrevSyncKey] = useState(`${activeProfileId}:${open}`)
  const [regionInput, setRegionInput] = useState(activeProfile?.region ?? '')
  const [daysInput, setDaysInput] = useState(
    activeProfile?.new_release_days !== null && activeProfile?.new_release_days !== undefined
      ? String(activeProfile.new_release_days)
      : '',
  )
  const [savingSettings, setSavingSettings] = useState(false)
  const [settingsError, setSettingsError] = useState<string | null>(null)
  const [settingsSaved, setSettingsSaved] = useState(false)
  const [syncPromptMessage, setSyncPromptMessage] = useState<string | null>(null)

  const currentSyncKey = `${activeProfileId}:${open}:${activeProfile?.region ?? ''}:${activeProfile?.new_release_days ?? ''}`
  if (currentSyncKey !== prevSyncKey) {
    setPrevSyncKey(currentSyncKey)
    setRegionInput(activeProfile?.region ?? '')
    setDaysInput(
      activeProfile?.new_release_days !== null && activeProfile?.new_release_days !== undefined
        ? String(activeProfile.new_release_days)
        : '',
    )
    setSettingsError(null)
    setSettingsSaved(false)
  }

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!newProfileName.trim()) return
    setCreating(true)
    try {
      await onCreateProfile(newProfileName.trim())
      setNewProfileName('')
    } finally {
      setCreating(false)
    }
  }

  const handleSaveSettings = async (e: React.FormEvent) => {
    e.preventDefault()
    setSettingsError(null)
    setSettingsSaved(false)

    const trimmedRegion = regionInput.trim().toUpperCase()
    if (trimmedRegion !== '' && !/^[A-Z]{2}$/.test(trimmedRegion)) {
      setSettingsError('Region must be a 2-letter ISO code (e.g. US, GB, CA) or blank for default.')
      return
    }

    let parsedDays: number | null = null
    if (daysInput.trim() !== '') {
      const n = Number(daysInput.trim())
      if (!Number.isInteger(n) || n < 1 || n > 365) {
        setSettingsError('New release days must be a whole number between 1 and 365.')
        return
      }
      parsedDays = n
    }

    const nextRegion = trimmedRegion === '' ? null : trimmedRegion
    const prevEffectiveRegion = activeProfile?.region ?? config?.region ?? 'US'
    const nextEffectiveRegion = nextRegion ?? config?.region ?? 'US'
    const prevEffectiveDays = activeProfile?.new_release_days ?? config?.new_release_days ?? 30
    const nextEffectiveDays = parsedDays ?? config?.new_release_days ?? 30

    setSavingSettings(true)
    try {
      await onUpdateProfileSettings(activeProfileId, {
        region: nextRegion,
        new_release_days: parsedDays,
      })
      setSettingsSaved(true)
      if (nextEffectiveRegion !== prevEffectiveRegion) {
        setSyncPromptMessage(
          `Region updated to ${nextEffectiveRegion}. Sync now to update streaming provider availability.`,
        )
      } else if (nextEffectiveDays > prevEffectiveDays) {
        setSyncPromptMessage(
          `Release window increased to ${nextEffectiveDays} days. Sync now to fetch older releases.`,
        )
      }
    } catch (err) {
      setSettingsError(err instanceof Error ? err.message : 'Failed to save settings')
    } finally {
      setSavingSettings(false)
    }
  }

  return (
    <Dialog open={open} onClose={onClose} title="Profiles & Preferences">
      <div className="space-y-6">
        {/* Profile Switcher & Creator */}
        <div className="space-y-3">
          <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            User Profiles
          </h4>

          <div className="space-y-2">
            {profiles.map((p) => (
              <div
                key={p.id}
                className={`flex items-center justify-between rounded-lg border p-2.5 transition ${
                  p.id === activeProfileId
                    ? 'border-accent bg-accent/10'
                    : 'border-border bg-muted/30 hover:bg-muted/50'
                }`}
              >
                <button
                  type="button"
                  onClick={() => onSelectProfile(p.id)}
                  className="flex-1 text-left"
                >
                  <div className="font-semibold text-foreground">
                    {p.name}{' '}
                    {p.id === activeProfileId && (
                      <span className="text-xs text-accent">(Active)</span>
                    )}
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    Created {new Date(p.created_at).toLocaleDateString()}
                  </div>
                </button>

                {p.id !== 'default' && (
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => onDeleteProfile(p.id)}
                    title="Delete profile"
                  >
                    <Trash2 className="h-4 w-4 text-red-400" />
                  </Button>
                )}
              </div>
            ))}
          </div>

          <form onSubmit={handleCreate} className="flex gap-2 pt-1">
            <Input
              value={newProfileName}
              onChange={(e) => setNewProfileName(e.target.value)}
              placeholder="New profile name..."
            />
            <Button
              type="submit"
              variant="default"
              size="sm"
              disabled={creating || !newProfileName.trim()}
            >
              <Plus className="h-4 w-4" />
              <span>Add</span>
            </Button>
          </form>
        </div>

        {/* Region & Release Window Settings */}
        <div className="space-y-3 border-t border-border pt-4">
          <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Region & Release Window ({activeProfile?.name ?? 'Default'})
          </h4>
          <p className="text-xs text-muted-foreground">
            Leave blank to use server defaults ({config?.region ?? 'US'},{' '}
            {config?.new_release_days ?? 30} days).
          </p>

          <form onSubmit={handleSaveSettings} className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div>
              <label
                htmlFor="profile-region-input"
                className="mb-1 block text-xs font-medium text-foreground"
              >
                Region (ISO 3166-1)
              </label>
              <Input
                id="profile-region-input"
                value={regionInput}
                onChange={(e) => {
                  setRegionInput(e.target.value)
                  setSettingsSaved(false)
                }}
                placeholder={config?.region ?? 'US'}
                maxLength={2}
              />
            </div>

            <div>
              <label
                htmlFor="profile-days-input"
                className="mb-1 block text-xs font-medium text-foreground"
              >
                New Release Days
              </label>
              <Input
                id="profile-days-input"
                type="number"
                min={1}
                max={365}
                value={daysInput}
                onChange={(e) => {
                  setDaysInput(e.target.value)
                  setSettingsSaved(false)
                }}
                placeholder={String(config?.new_release_days ?? 30)}
              />
            </div>

            <div className="flex items-end">
              <Button type="submit" variant="default" size="sm" disabled={savingSettings}>
                {savingSettings ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </form>

          {settingsError && <p className="text-xs text-red-400">{settingsError}</p>}
          {settingsSaved && <p className="text-xs text-emerald-400">Settings saved.</p>}

          {syncPromptMessage && (
            <div className="flex items-center justify-between gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-amber-200">
              <span>{syncPromptMessage}</span>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={async () => {
                  await onTriggerSync()
                  setSyncPromptMessage(null)
                }}
              >
                <RefreshCw className="h-3 w-3" />
                <span>Sync Now</span>
              </Button>
            </div>
          )}
        </div>

        {/* Hidden Titles Section */}
        <div className="space-y-3 border-t border-border pt-4">
          <div className="flex items-center justify-between">
            <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Hidden Titles ({activeProfile?.name ?? 'Default'})
            </h4>
            <span className="text-xs text-muted-foreground">
              {hiddenState?.hidden_titles.length ?? 0} hidden
            </span>
          </div>

          {hiddenState && hiddenState.hidden_titles.length > 0 ? (
            <div className="max-h-48 space-y-1.5 overflow-y-auto pr-1">
              {hiddenState.hidden_titles.map((t) => (
                <div
                  key={t.program_id}
                  className="flex items-center justify-between rounded-md border border-border bg-muted/30 px-3 py-1.5 text-xs"
                >
                  <span className="truncate font-medium text-foreground">{t.title}</span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => onUnhideTitle(t.program_id)}
                    title="Unhide title"
                  >
                    <Undo className="h-3 w-3" />
                    <span>Unhide</span>
                  </Button>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-xs italic text-muted-foreground">
              No hidden titles for this profile.
            </p>
          )}
        </div>

        {/* Hidden Genres Section */}
        <div className="space-y-3 border-t border-border pt-4">
          <div className="flex items-center justify-between">
            <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Hidden Genres ({activeProfile?.name ?? 'Default'})
            </h4>
            <span className="text-xs text-muted-foreground">
              {hiddenState?.hidden_genres.length ?? 0} hidden
            </span>
          </div>

          {hiddenState && hiddenState.hidden_genres.length > 0 ? (
            <div className="flex flex-wrap gap-1.5">
              {hiddenState.hidden_genres.map((g) => (
                <span
                  key={g}
                  className="inline-flex items-center gap-1 rounded-full border border-border bg-muted/60 pl-2.5 pr-1.5 py-0.5 text-xs text-foreground"
                >
                  <span>{g}</span>
                  <button
                    type="button"
                    onClick={() => onUnhideGenre(g)}
                    className="rounded-full p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
                    title={`Unhide ${g}`}
                  >
                    <Undo className="h-3 w-3" />
                  </button>
                </span>
              ))}
            </div>
          ) : (
            <p className="text-xs italic text-muted-foreground">
              No hidden genres. You can hide a genre from any title detail modal.
            </p>
          )}
        </div>
      </div>
    </Dialog>
  )
}
