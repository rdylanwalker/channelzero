import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { Profile, PublicConfig, SyncStatusResponse } from '../../types'
import { Header } from '../Header'

const baseConfig: PublicConfig = {
  region: 'US',
  timezone: 'UTC',
  new_release_days: 30,
  refresh_interval_hours: 6,
  providers: [],
}

const baseProfiles: Profile[] = [
  {
    id: 'default',
    name: 'Default',
    region: null,
    new_release_days: null,
    created_at: '2026-01-01T00:00:00Z',
  },
  {
    id: 'uk',
    name: 'UK Profile',
    region: 'GB',
    new_release_days: 14,
    created_at: '2026-01-02T00:00:00Z',
  },
]

describe('Header', () => {
  it('shows server default region and release days when profile overrides are null', () => {
    render(
      <Header
        profiles={baseProfiles}
        activeProfileId="default"
        onSelectProfile={vi.fn()}
        config={baseConfig}
        syncStatus={null}
        onRefresh={vi.fn()}
        onOpenSyncStatus={vi.fn()}
        onOpenProfileSettings={vi.fn()}
      />,
    )
    expect(screen.getByText('US · 30d')).toBeInTheDocument()
  })

  it('shows profile-specific region and release days when set', () => {
    render(
      <Header
        profiles={baseProfiles}
        activeProfileId="uk"
        onSelectProfile={vi.fn()}
        config={baseConfig}
        syncStatus={null}
        onRefresh={vi.fn()}
        onOpenSyncStatus={vi.fn()}
        onOpenProfileSettings={vi.fn()}
      />,
    )
    expect(screen.getByText('GB · 14d')).toBeInTheDocument()
  })

  it('handles profile selection, modal triggers, refresh, and syncing/error states', () => {
    const onSelectProfile = vi.fn()
    const onRefresh = vi.fn()
    const onOpenSyncStatus = vi.fn()
    const onOpenProfileSettings = vi.fn()

    const syncingStatus: SyncStatusResponse = {
      last_sync_at: null,
      is_syncing: true,
      total_programs: 10,
      refresh_interval_hours: 6,
      new_release_days: 30,
      region: 'US',
      sources: [
        {
          name: 'TVmaze',
          tier: 'anonymous',
          enabled: true,
          configured: true,
          last_sync_at: null,
          last_status: 'error',
          last_error: 'Timeout',
          items_fetched: 0,
        },
      ],
    }

    render(
      <Header
        profiles={baseProfiles}
        activeProfileId="default"
        onSelectProfile={onSelectProfile}
        config={baseConfig}
        syncStatus={syncingStatus}
        onRefresh={onRefresh}
        onOpenSyncStatus={onOpenSyncStatus}
        onOpenProfileSettings={onOpenProfileSettings}
      />,
    )

    fireEvent.change(screen.getByLabelText('Active profile'), { target: { value: 'uk' } })
    expect(onSelectProfile).toHaveBeenCalledWith('uk')

    fireEvent.click(screen.getByTitle('Profiles & Hidden Titles/Genres'))
    expect(onOpenProfileSettings).toHaveBeenCalledTimes(1)

    fireEvent.click(screen.getByTitle('API & Sync Status'))
    expect(onOpenSyncStatus).toHaveBeenCalledTimes(1)

    const refreshBtn = screen.getByTitle('Refresh metadata now')
    expect(refreshBtn).toBeDisabled()
    expect(screen.getByText('Syncing...')).toBeInTheDocument()
  })

  it('calls onRefresh when not syncing', () => {
    const onRefresh = vi.fn()
    render(
      <Header
        profiles={baseProfiles}
        activeProfileId="default"
        onSelectProfile={vi.fn()}
        config={baseConfig}
        syncStatus={null}
        onRefresh={onRefresh}
        onOpenSyncStatus={vi.fn()}
        onOpenProfileSettings={vi.fn()}
      />,
    )

    fireEvent.click(screen.getByTitle('Refresh metadata now'))
    expect(onRefresh).toHaveBeenCalledTimes(1)
  })
})
