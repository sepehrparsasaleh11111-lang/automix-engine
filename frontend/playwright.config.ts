import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 30_000,
  use: { baseURL: 'http://localhost:1420' },
  webServer: {
    command: 'pnpm dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    env: { VITE_E2E: 'true' },
  },
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }],
});