import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  outputDir: './e2e/tmp/test-results',

  use: {
    baseURL: 'http://localhost:8090',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],

  // Run the backend server (serves both API and frontend static files)
  webServer: {
    command: 'npm run build && cd .. && cargo run -- serve --listen 0.0.0.0:8090',
    url: 'http://localhost:8090',
    reuseExistingServer: !process.env.CI,
    timeout: 300000, // 5 minutes for backend compilation
  },
})
