import { defineConfig } from '@playwright/test';

const port = process.env.E2E_PORT ?? '8099';
const baseURL = `http://127.0.0.1:${port}`;

// Runs against the real `web` binary (built by `cargo run`), with the component gallery enabled.
// Uses the machine's Chrome (`channel: 'chrome'`), so no browser download is needed in CI.
export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['github'], ['list']] : 'list',
  use: { baseURL, channel: 'chrome', trace: 'retain-on-failure' },
  projects: [
    { name: 'desktop', use: { viewport: { width: 1280, height: 800 } } },
    {
      name: 'mobile',
      use: { viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true },
    },
  ],
  webServer: {
    command: 'cargo run --locked --quiet -p web',
    cwd: '..',
    url: `${baseURL}/healthz`,
    env: {
      WEB_BIND: `127.0.0.1:${port}`,
      METRICS_BIND: `127.0.0.1:${Number(port) + 1000}`,
      WEB_UI_GALLERY: '1',
      RUST_LOG: 'warn',
    },
    reuseExistingServer: !process.env.CI,
    timeout: 300_000,
  },
});
