import { expect, test } from '@playwright/test'

test.describe('ChannelZero Dashboard E2E', () => {
  test('loads dashboard with title, favicon, and seeded releases', async ({ page }) => {
    await page.goto('/')
    await expect(page).toHaveTitle(/ChannelZero/)
    await expect(page.getByRole('heading', { name: 'ChannelZero' })).toBeVisible()

    const faviconHref = await page.locator('link[rel="icon"]').getAttribute('href')
    expect(faviconHref).toBe('/favicon.svg')

    await expect(page.getByRole('heading', { name: 'Neon Horizon' })).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Laugh Track' })).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Deep Orbit' })).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Old Classic' })).toBeVisible()
    await expect(page.getByText('4 programs')).toBeVisible()
  })

  test('searches, filters, sorts, and preserves full genre dropdown options', async ({ page }) => {
    await page.goto('/')
    await expect(page.getByText('4 programs')).toBeVisible()

    // Search filter
    const searchInput = page.getByPlaceholder('Search titles or synopses...')
    await searchInput.fill('Neon')
    await expect(page.getByText('1 program')).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Neon Horizon' })).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Laugh Track' })).toBeHidden()
    await searchInput.fill('')
    await expect(page.getByText('4 programs')).toBeVisible()

    // Genre filter bug regression: selecting Comedy must not remove Sci-Fi or Drama from dropdown
    const genreSelect = page.locator('select', { has: page.locator('option[value="Comedy"]') })
    await genreSelect.selectOption('Comedy')
    await expect(page.getByText('1 program')).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Laugh Track' })).toBeVisible()
    await expect(genreSelect.locator('option[value="Sci-Fi"]')).toBeAttached()
    await expect(genreSelect.locator('option[value="Drama"]')).toBeAttached()
    await genreSelect.selectOption('all')
    await expect(page.getByText('4 programs')).toBeVisible()

    // Provider pill filter
    await page.getByRole('button', { name: 'Netflix' }).click()
    await expect(page.getByText('1 program')).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Neon Horizon' })).toBeVisible()
    await page.getByRole('button', { name: 'All' }).click()

    // Sort by rating (highest first -> Deep Orbit 9.1)
    const sortSelect = page.locator('select', { has: page.locator('option[value="rating"]') })
    await sortSelect.selectOption('rating')
    const firstCardHeading = page.locator('main h3').first()
    await expect(firstCardHeading).toHaveText('Deep Orbit')
  })

  test('persists watchlist and watched interactions across reloads', async ({ page }) => {
    await page.goto('/')

    await page.getByRole('heading', { name: 'Neon Horizon' }).click()
    await expect(
      page.getByText('A cyberpunk courier races across a flooded megacity.'),
    ).toBeVisible()

    await page.getByRole('button', { name: 'Save to Watchlist', exact: true }).click()
    await page.getByRole('button', { name: 'Mark Watched', exact: true }).click()
    await page.keyboard.press('Escape')

    const neonCard = page.locator('article.group', { hasText: 'Neon Horizon' }).first()
    await expect(neonCard.getByText('Saved')).toBeVisible()
    await expect(neonCard.getByText('Watched')).toBeVisible()

    await page.reload()
    await expect(neonCard.getByText('Saved')).toBeVisible()
    await expect(neonCard.getByText('Watched')).toBeVisible()
  })

  test('hides and unhides titles and genres via Preferences', async ({ page }) => {
    await page.goto('/')

    // Hide a title via card action
    const laughCard = page.locator('article.group', { hasText: 'Laugh Track' }).first()
    await laughCard.getByTitle('Not interested (hide title)').click()
    await expect(page.getByRole('heading', { name: 'Laugh Track' })).toBeHidden()

    // Hide a genre via DetailModal
    await page.getByRole('heading', { name: 'Neon Horizon' }).click()
    await page.getByTitle('Hide all "Sci-Fi" titles from feed').click()
    await page.keyboard.press('Escape')

    await expect(page.getByRole('heading', { name: 'Neon Horizon' })).toBeHidden()
    await expect(page.getByRole('heading', { name: 'Deep Orbit' })).toBeHidden()
    await expect(page.getByRole('heading', { name: 'Old Classic' })).toBeVisible()

    // Unhide both in Preferences
    await page.getByRole('button', { name: /Preferences/i }).click()
    await expect(page.getByText('Laugh Track')).toBeVisible()
    await page.getByTitle('Unhide title').first().click()
    await page.getByTitle('Unhide Sci-Fi').click()
    await page.keyboard.press('Escape')

    await expect(page.getByRole('heading', { name: 'Neon Horizon' })).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Laugh Track' })).toBeVisible()
  })

  test('creates a profile, updates region & new_release_days, and deletes profile', async ({
    page,
  }) => {
    await page.goto('/')

    await page.getByRole('button', { name: /Preferences/i }).click()
    await page.getByPlaceholder('New profile name...').fill('Alex')
    await page.getByRole('button', { name: 'Add' }).click()
    await expect(page.getByText('Alex (Active)')).toBeVisible()

    await page.getByLabel('Region (ISO 3166-1)').fill('GB')
    await page.getByLabel('New Release Days').fill('14')
    await page.getByRole('button', { name: 'Save', exact: true }).click()

    await expect(page.getByText(/Region updated to/i)).toBeVisible()
    await page.keyboard.press('Escape')

    await expect(page.getByText('GB · 14d')).toBeVisible()
    // Old Classic (20 days ago) should be excluded by 14-day window
    await expect(page.getByRole('heading', { name: 'Old Classic' })).toBeHidden()
    await expect(page.getByRole('heading', { name: 'Neon Horizon' })).toBeVisible()

    // Clean up profile
    await page.getByRole('button', { name: /Preferences/i }).click()
    await page.getByTitle('Delete profile').click()
    await expect(page.getByText('Default (Active)')).toBeVisible()
  })

  test('opens API & Sync Status modal', async ({ page }) => {
    await page.goto('/')
    await page.getByRole('button', { name: /Sources/i }).click()
    await expect(page.getByText('Metadata Sources & Sync Status')).toBeVisible()
    await expect(page.getByText('Total Titles')).toBeVisible()
  })
})
