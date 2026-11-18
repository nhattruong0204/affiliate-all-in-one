import { expect, test } from '@playwright/test';
import { PAGES } from './helpers';

test('navigation switches from bottom bar to top bar at the lg breakpoint', async ({ page }) => {
  await page.goto('/');
  await page.setViewportSize({ width: 1023, height: 800 });
  await expect(page.locator('.bottom-nav')).toBeVisible();
  await expect(page.locator('.desktop-nav')).toBeHidden();

  await page.setViewportSize({ width: 1024, height: 800 });
  await expect(page.locator('.bottom-nav')).toBeHidden();
  await expect(page.locator('.desktop-nav')).toBeVisible();
  await expect(page.locator('.site-header .btn-primary')).toBeVisible();
});

test('the current page is marked in whichever navigation is visible', async ({ page }) => {
  await page.goto('/');
  const visible = page.locator('.desktop-nav:visible, .bottom-nav:visible');
  await expect(visible.locator('[aria-current="page"]')).toHaveCount(1);
});

for (const [path] of PAGES) {
  test(`no horizontal overflow on ${path}`, async ({ page }) => {
    await page.goto(path);
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(overflow).toBeLessThanOrEqual(0);
  });
}

test('interactive controls are comfortable touch targets', async ({ page }) => {
  await page.goto('/_ui');
  const tooSmall = await page.evaluate(() => {
    const minimum = (el: Element) =>
      el.matches('.bottom-nav-link, .icon-button, .btn-md, .btn-lg, .toast-close') ? 44 : 24;
    return [...document.querySelectorAll('button, a.btn, .bottom-nav-link, .nav-link, .lang-switch a')]
      .filter((el) => (el as HTMLElement).offsetParent !== null || getComputedStyle(el).position === 'fixed')
      .map((el) => {
        const box = el.getBoundingClientRect();
        return { el: `${el.tagName}.${el.className}`, w: box.width, h: box.height, min: minimum(el) };
      })
      .filter((b) => b.w > 0 && (b.h < b.min || b.w < 24));
  });
  expect(tooSmall).toEqual([]);
});

test('skip link is the first tab stop, becomes visible and moves focus to main', async ({ page }) => {
  await page.goto('/');
  await page.keyboard.press('Tab');
  const skip = page.locator('.skip-link');
  await expect(skip).toBeFocused();
  const box = await skip.boundingBox();
  expect(box && box.y >= 0).toBeTruthy();
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/#main$/);
  await expect(page.locator('#main')).toBeFocused();
});

test('focus rings are visible on buttons and inputs', async ({ page }) => {
  await page.goto('/_ui');
  await page.keyboard.press('Tab'); // enter keyboard modality
  const button = page.locator('main .btn-primary').first();
  await button.focus();
  const outline = await button.evaluate((el) => {
    const s = getComputedStyle(el);
    return { style: s.outlineStyle, width: Number.parseFloat(s.outlineWidth) };
  });
  expect(outline.style).not.toBe('none');
  expect(outline.width).toBeGreaterThanOrEqual(2);
});

test('landmarks: one banner, one main, one contentinfo, h1 per page', async ({ page }) => {
  for (const [path] of PAGES) {
    await page.goto(path);
    await expect(page.getByRole('banner')).toHaveCount(1);
    await expect(page.getByRole('main')).toHaveCount(1);
    await expect(page.getByRole('contentinfo')).toHaveCount(1);
    await expect(page.getByRole('heading', { level: 1 })).toHaveCount(1);
  }
});
