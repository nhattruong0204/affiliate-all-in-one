import { expect, test } from '@playwright/test';

test.use({ javaScriptEnabled: false });

test('without scripting the page renders, navigates and hides controls that cannot work', async ({
  page,
}) => {
  await page.goto('/_ui');
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  await expect(page.locator('[data-theme-toggle]')).toBeHidden();
  await expect(page.locator('[data-toast-demo]').first()).toBeHidden();

  await page.locator('[data-lang-link="en"]').click();
  await expect(page).toHaveURL(/\/en\/_ui$/);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('UI gallery');
});

test('a saved dark theme still renders without scripting', async ({ page, context, baseURL }) => {
  await context.addCookies([{ name: 'aao_theme', value: 'dark', url: baseURL! }]);
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  expect(await page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe(
    'rgb(11, 18, 32)',
  );
});
