import { test } from '@playwright/test';
import { setThemeCookie, type Theme } from './helpers';

// Evidence for the PR (desktop + mobile, light + dark). Only runs with SCREENSHOTS=1.
test.skip(!process.env.SCREENSHOTS, 'set SCREENSHOTS=1 to capture');

const dir = process.env.SCREENSHOT_DIR ?? 'screenshots';
const shots: ReadonlyArray<readonly [string, string, boolean]> = [
  ['shell-vi', '/', false],
  ['shell-en', '/en', false],
  ['gallery', '/_ui', true],
  ['not-found', '/khong-co-trang', false],
];

for (const theme of ['light', 'dark'] as Theme[]) {
  for (const [name, path, fullPage] of shots) {
    test(`screenshot ${name} ${theme}`, async ({ page, context, baseURL }, info) => {
      await setThemeCookie(context, baseURL!, theme);
      await page.goto(path);
      await page.evaluate(() => document.fonts.ready);
      await page.screenshot({
        path: `${dir}/${info.project.name}-${theme}-${name}.png`,
        fullPage,
        animations: 'disabled',
      });
    });
  }

  test(`screenshot toasts ${theme}`, async ({ page, context, baseURL }, info) => {
    await setThemeCookie(context, baseURL!, theme);
    await page.goto('/_ui#g-toasts');
    for (const kind of ['success', 'warning', 'error'])
      await page.locator(`[data-toast-demo="${kind}"]`).click();
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({ path: `${dir}/${info.project.name}-${theme}-toasts.png`, animations: 'disabled' });
  });
}
