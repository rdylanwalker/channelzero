import { mkdirSync } from 'node:fs'
import path from 'node:path'
import { expect, test } from '@playwright/test'

test('capture dashboard screenshot for README', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')

  await expect(page.getByRole('heading', { name: 'ChannelZero' })).toBeVisible()
  await expect(page.locator('article.group').first()).toBeVisible({ timeout: 30_000 })

  // Wait for all visible poster images to finish loading
  await page.evaluate(async () => {
    const imgs = Array.from(document.images)
    await Promise.all(
      imgs
        .filter((img) => !img.complete)
        .map(
          (img) =>
            new Promise<void>((resolve) => {
              img.addEventListener('load', () => resolve(), { once: true })
              img.addEventListener('error', () => resolve(), { once: true })
            }),
        ),
    )
  })
  await page.waitForTimeout(300)

  const outPath = path.resolve(process.cwd(), '../docs/dashboard.png')
  mkdirSync(path.dirname(outPath), { recursive: true })
  await page.screenshot({ path: outPath, fullPage: false })
})
