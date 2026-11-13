import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { SyncStatusResponse } from '../../types'
import { SyncStatusModal } from '../SyncStatusModal'

const baseStatus: SyncStatusResponse = {
  last_sync_at: '2026-02-10T12:00:00Z',
  is_syncing: false,
  total_programs: 142,
  refresh_interval_hours: 6,
  new_release_days: 30,
  region: 'US',
  sources: [
    {
      name: 'TVmaze',
      tier: 'anonymous',
      enabled: true,
      configured: true,
      last_sync_at: '2026-02-10T12:00:00Z',
      last_status: 'ok',
      last_error: null,
      items_fetched: 80,
    },
    {
      name: 'TMDB',
      tier: 'free_key',
      enabled: true,
      configured: false,
      last_sync_at: null,
      last_status: 'unconfigured',
      last_error: null,
      items_fetched: 0,
    },
    {
      name: 'OMDb',
      tier: 'free_key',
      enabled: true,
      configured: true,
      last_sync_at: '2026-02-10T11:55:00Z',
      last_status: 'error',
      last_error: 'Daily rate limit reached',
      items_fetched: 12,
    },
    {
      name: 'Watchmode',
      tier: 'paid',
      enabled: false,
      configured: false,
      last_sync_at: null,
      last_status: 'disabled',
      last_error: null,
      items_fetched: 0,
    },
  ],
}

describe('SyncStatusModal', () => {
  it('renders nothing when status is null', () => {
    const { container } = render(
      <SyncStatusModal open={true} onClose={vi.fn()} status={null} onTriggerRefresh={vi.fn()} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('renders summary metrics, all source statuses, and triggers refresh', () => {
    const onTriggerRefresh = vi.fn()
    render(
      <SyncStatusModal
        open={true}
        onClose={vi.fn()}
        status={baseStatus}
        onTriggerRefresh={onTriggerRefresh}
      />,
    )

    expect(screen.getByText('US')).toBeInTheDocument()
    expect(screen.getByText('142')).toBeInTheDocument()
    expect(screen.getByText('30 days')).toBeInTheDocument()
    expect(screen.getByText('Every 6h')).toBeInTheDocument()

    expect(screen.getByText('Active')).toBeInTheDocument()
    expect(screen.getByText('Key Required')).toBeInTheDocument()
    expect(screen.getByText('Sync Error')).toBeInTheDocument()
    expect(screen.getByText('Daily rate limit reached')).toBeInTheDocument()
    expect(screen.getByText('Disabled')).toBeInTheDocument()
    expect(screen.getByText('80 items processed')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: /Sync Now/i }))
    expect(onTriggerRefresh).toHaveBeenCalledTimes(1)
  })

  it('shows Never when last_sync_at is null and disables button while syncing', () => {
    render(
      <SyncStatusModal
        open={true}
        onClose={vi.fn()}
        status={{ ...baseStatus, last_sync_at: null, is_syncing: true }}
        onTriggerRefresh={vi.fn()}
      />,
    )

    expect(screen.getByText('Never')).toBeInTheDocument()
    const syncBtn = screen.getByRole('button', { name: /Syncing.../i })
    expect(syncBtn).toBeDisabled()
  })
})
