import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { Profile, PublicConfig } from '../../types'
import { ProfileSettingsModal } from '../ProfileSettingsModal'

const config: PublicConfig = {
  region: 'US',
  timezone: 'UTC',
  new_release_days: 30,
  refresh_interval_hours: 6,
  providers: [],
}

const profiles: Profile[] = [
  {
    id: 'default',
    name: 'Default',
    region: null,
    new_release_days: null,
    created_at: '2026-01-01T00:00:00Z',
  },
]

describe('ProfileSettingsModal', () => {
  it('validates region format and prompts to sync after region change', async () => {
    const onUpdateProfileSettings = vi.fn().mockResolvedValue(undefined)
    const onTriggerSync = vi.fn().mockResolvedValue(undefined)

    render(
      <ProfileSettingsModal
        open={true}
        onClose={vi.fn()}
        profiles={profiles}
        activeProfileId="default"
        config={config}
        onSelectProfile={vi.fn()}
        onCreateProfile={vi.fn()}
        onDeleteProfile={vi.fn()}
        onUpdateProfileSettings={onUpdateProfileSettings}
        onTriggerSync={onTriggerSync}
        hiddenState={{ hidden_titles: [], hidden_genres: [] }}
        onUnhideTitle={vi.fn()}
        onUnhideGenre={vi.fn()}
      />,
    )

    const regionInput = screen.getByLabelText('Region (ISO 3166-1)')
    const daysInput = screen.getByLabelText('New Release Days')
    const saveButton = screen.getByRole('button', { name: 'Save' })

    fireEvent.change(regionInput, { target: { value: 'GB' } })
    fireEvent.change(daysInput, { target: { value: '14' } })
    fireEvent.click(saveButton)

    await waitFor(() => {
      expect(onUpdateProfileSettings).toHaveBeenCalledWith('default', {
        region: 'GB',
        new_release_days: 14,
      })
    })

    expect(await screen.findByText('Settings saved.')).toBeInTheDocument()
    expect(await screen.findByText(/Region updated to/i)).toBeInTheDocument()

    const syncNowButton = screen.getByRole('button', { name: /Sync Now/i })
    fireEvent.click(syncNowButton)
    await waitFor(() => {
      expect(onTriggerSync).toHaveBeenCalledTimes(1)
    })
  })

  it('supports creating, selecting, and deleting profiles, plus unhiding titles and genres', async () => {
    const onSelectProfile = vi.fn()
    const onCreateProfile = vi.fn().mockResolvedValue(undefined)
    const onDeleteProfile = vi.fn().mockResolvedValue(undefined)
    const onUnhideTitle = vi.fn().mockResolvedValue(undefined)
    const onUnhideGenre = vi.fn().mockResolvedValue(undefined)

    const multiProfiles: Profile[] = [
      ...profiles,
      {
        id: 'kids',
        name: 'Kids',
        region: 'CA',
        new_release_days: 7,
        created_at: '2026-01-03T00:00:00Z',
      },
    ]

    render(
      <ProfileSettingsModal
        open={true}
        onClose={vi.fn()}
        profiles={multiProfiles}
        activeProfileId="kids"
        config={config}
        onSelectProfile={onSelectProfile}
        onCreateProfile={onCreateProfile}
        onDeleteProfile={onDeleteProfile}
        onUpdateProfileSettings={vi.fn()}
        onTriggerSync={vi.fn()}
        hiddenState={{
          hidden_titles: [
            {
              program_id: 'p1',
              title: 'Hidden Show',
              media_type: 'series',
              poster_url: null,
              updated_at: '2026-01-04T00:00:00Z',
            },
          ],
          hidden_genres: ['Horror'],
        }}
        onUnhideTitle={onUnhideTitle}
        onUnhideGenre={onUnhideGenre}
      />,
    )

    // Select default profile
    fireEvent.click(screen.getByText('Default'))
    expect(onSelectProfile).toHaveBeenCalledWith('default')

    // Delete non-default profile
    fireEvent.click(screen.getByTitle('Delete profile'))
    expect(onDeleteProfile).toHaveBeenCalledWith('kids')

    // Create new profile
    const nameInput = screen.getByPlaceholderText('New profile name...')
    fireEvent.change(nameInput, { target: { value: '  Guest  ' } })
    fireEvent.click(screen.getByRole('button', { name: 'Add' }))
    await waitFor(() => {
      expect(onCreateProfile).toHaveBeenCalledWith('Guest')
    })

    // Unhide title and genre
    fireEvent.click(screen.getByTitle('Unhide title'))
    expect(onUnhideTitle).toHaveBeenCalledWith('p1')

    fireEvent.click(screen.getByTitle('Unhide Horror'))
    expect(onUnhideGenre).toHaveBeenCalledWith('Horror')
  })

  it('validates invalid region, invalid days, and displays server errors', async () => {
    const onUpdateProfileSettings = vi.fn().mockRejectedValue(new Error('Server rejected'))

    render(
      <ProfileSettingsModal
        open={true}
        onClose={vi.fn()}
        profiles={profiles}
        activeProfileId="default"
        config={config}
        onSelectProfile={vi.fn()}
        onCreateProfile={vi.fn()}
        onDeleteProfile={vi.fn()}
        onUpdateProfileSettings={onUpdateProfileSettings}
        onTriggerSync={vi.fn()}
        hiddenState={null}
        onUnhideTitle={vi.fn()}
        onUnhideGenre={vi.fn()}
      />,
    )

    const regionInput = screen.getByLabelText('Region (ISO 3166-1)')
    const daysInput = screen.getByLabelText('New Release Days')
    const saveButton = screen.getByRole('button', { name: 'Save' })

    // Invalid region
    fireEvent.change(regionInput, { target: { value: '1' } })
    fireEvent.click(saveButton)
    expect(await screen.findByText(/Region must be a 2-letter ISO code/i)).toBeInTheDocument()
    expect(onUpdateProfileSettings).not.toHaveBeenCalled()

    // Invalid days (submit form directly to bypass native HTML5 max=365 constraint)
    fireEvent.change(regionInput, { target: { value: '' } })
    fireEvent.change(daysInput, { target: { value: '400' } })
    fireEvent.submit(saveButton.closest('form') as HTMLFormElement)
    expect(
      await screen.findByText(/New release days must be a whole number between 1 and 365/i),
    ).toBeInTheDocument()

    // Valid blank values -> triggers onUpdateProfileSettings and catches error
    fireEvent.change(daysInput, { target: { value: '' } })
    fireEvent.click(saveButton)
    expect(await screen.findByText('Server rejected')).toBeInTheDocument()
  })

  it('prompts to sync when new_release_days is increased', async () => {
    const onUpdateProfileSettings = vi.fn().mockResolvedValue(undefined)

    render(
      <ProfileSettingsModal
        open={true}
        onClose={vi.fn()}
        profiles={profiles}
        activeProfileId="default"
        config={config}
        onSelectProfile={vi.fn()}
        onCreateProfile={vi.fn()}
        onDeleteProfile={vi.fn()}
        onUpdateProfileSettings={onUpdateProfileSettings}
        onTriggerSync={vi.fn()}
        hiddenState={null}
        onUnhideTitle={vi.fn()}
        onUnhideGenre={vi.fn()}
      />,
    )

    fireEvent.change(screen.getByLabelText('New Release Days'), { target: { value: '60' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByText(/Release window increased to 60 days/i)).toBeInTheDocument()
  })
})
