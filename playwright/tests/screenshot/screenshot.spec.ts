import { mkdirSync } from 'node:fs'
import path from 'node:path'
import { expect, test } from '@playwright/test'

test('capture dashboard screenshot for README', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')

  await expect(page.getByRole('heading', { name: 'ChannelZero' })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Neon Horizon' })).toBeVisible()
  await expect(page.getByText('4 programs')).toBeVisible()
  await page.waitForLoadState('networkidle')

  const outPath = path.resolve(process.cwd(), '../docs/dashboard.png')
  mkdirSync(path.dirname(outPath), { recursive: true })
  await page.screenshot({ path: outPath, fullPage: false })
})
