import { expect, test } from '@playwright/test';
import { DARK_BG, LIGHT_BG, cookieValue, pageBackground, setThemeCookie } from './helpers';

test('follows the OS preference until the user chooses', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.goto('/');
  await expect(page.locator('html')).not.toHaveAttribute('data-theme', /.*/);
  expect(await pageBackground(page)).toBe(DARK_BG);
  await expect(page.locator('[data-theme-toggle]')).toHaveAttribute('aria-pressed', 'true');

  await page.emulateMedia({ colorScheme: 'light' });
  expect(await pageBackground(page)).toBe(LIGHT_BG);
  await expect(page.locator('[data-theme-toggle]')).toHaveAttribute('aria-pressed', 'false');
});

test('toggle switches theme, stores the choice and the server renders it on reload', async ({
  page,
  context,
}) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await page.goto('/');
  const toggle = page.locator('[data-theme-toggle]');
  await expect(toggle).toBeVisible();
  await expect(toggle).toHaveAccessibleName('Đổi giao diện sáng/tối');

  await toggle.click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(toggle).toHaveAttribute('aria-pressed', 'true');
  expect(await cookieValue(context, 'aao_theme')).toBe('dark');
  expect(await pageBackground(page)).toBe(DARK_BG);

  // No flash: the first byte of the next response already carries the theme (no script needed).
  const html = await (await page.request.get('/')).text();
  expect(html).toContain('data-theme="dark"');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  expect(await pageBackground(page)).toBe(DARK_BG);

  await toggle.click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  expect(await cookieValue(context, 'aao_theme')).toBe('light');
  expect(await pageBackground(page)).toBe(LIGHT_BG);
});

test('an explicit choice beats the OS preference in both directions', async ({
  page,
  context,
  baseURL,
}) => {
  await page.emulateMedia({ colorScheme: 'dark' });
  await setThemeCookie(context, baseURL!, 'light');
  await page.goto('/');
  expect(await pageBackground(page)).toBe(LIGHT_BG);

  await page.emulateMedia({ colorScheme: 'light' });
  await setThemeCookie(context, baseURL!, 'dark');
  await page.reload();
  expect(await pageBackground(page)).toBe(DARK_BG);
});

test('theme persists while navigating between pages', async ({ page, context, baseURL }) => {
  await setThemeCookie(context, baseURL!, 'dark');
  for (const path of ['/', '/en', '/_ui', '/missing']) {
    await page.goto(path);
    expect(await pageBackground(page), path).toBe(DARK_BG);
  }
});
