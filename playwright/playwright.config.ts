import { defineConfig, devices } from '@playwright/test'

const isCI = Boolean(process.env.CI)

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  forbidOnly: isCI,
  retries: isCI ? 2 : 0,
  workers: 1,
  reporter: [['html', { open: 'never' }], ['list']],
  outputDir: 'results',
  use: {
    baseURL: process.env.CHANNELZERO_BASE_URL ?? 'http://127.0.0.1:8080',
    trace: 'on-first-retry',
  },
  projects: isCI
    ? [
        {
          name: 'chromium',
          use: { ...devices['Desktop Chrome'] },
        },
      ]
    : [
        {
          name: 'chromium',
          use: { ...devices['Desktop Chrome'] },
        },
        {
          name: 'firefox',
          use: { ...devices['Desktop Firefox'] },
        },
        {
          name: 'webkit',
          use: { ...devices['Desktop Safari'] },
        },
      ],
  webServer: process.env.CHANNELZERO_BASE_URL
    ? undefined
    : {
        command: 'node start-server.mjs',
        url: 'http://127.0.0.1:8080/api/health',
        reuseExistingServer: !isCI,
        timeout: 120_000,
      },
})
