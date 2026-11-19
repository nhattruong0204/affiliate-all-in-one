import { expect, test } from '@playwright/test';
import { cookieValue } from './helpers';

test('language switch keeps the page, localizes the UI and remembers the choice', async ({
  page,
  context,
}) => {
  await page.goto('/_ui');
  await expect(page.locator('html')).toHaveAttribute('lang', 'vi');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Thư viện giao diện');

  const current = page.locator('.lang-switch [aria-current="true"]');
  await expect(current).toHaveText('VI');

  await page.locator('[data-lang-link="en"]').click();
  await expect(page).toHaveURL(/\/en\/_ui$/);
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('UI gallery');
  await expect(page.locator('.lang-switch [aria-current="true"]')).toHaveText('EN');
  expect(await cookieValue(context, 'aao_lang')).toBe('en');

  // A remembered English choice sends the bare root to /en.
  await page.goto('/');
  await expect(page).toHaveURL(/\/en$/);
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');

  // Choosing Vietnamese again clears the redirect.
  await page.locator('[data-lang-link="vi"]').click();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('html')).toHaveAttribute('lang', 'vi');
  expect(await cookieValue(context, 'aao_lang')).toBe('vi');
});

test('both languages are complete: no untranslated keys leak into the page', async ({ page }) => {
  for (const path of ['/', '/en', '/_ui', '/en/_ui', '/nope', '/en/nope']) {
    await page.goto(path);
    const text = await page.evaluate(() => {
      const copy = document.body.cloneNode(true) as HTMLElement;
      // Design-token names in the gallery legend are intentionally kebab-case.
      copy.querySelector('[aria-labelledby="g-colors"]')?.remove();
      return copy.textContent ?? '';
    });
    expect(text, path).not.toMatch(/\b[a-z]+(-[a-z0-9]+)+\b/);
  }
});

test('language links expose hreflang and are real links (work without scripting)', async ({
  page,
}) => {
  await page.goto('/hoi-dap-khong-co');
  const links = page.locator('.lang-switch a');
  await expect(links).toHaveCount(2);
  await expect(links.nth(0)).toHaveAttribute('hreflang', 'vi');
  await expect(links.nth(1)).toHaveAttribute('hreflang', 'en');
  await expect(links.nth(1)).toHaveAttribute('href', '/en/hoi-dap-khong-co');
});
