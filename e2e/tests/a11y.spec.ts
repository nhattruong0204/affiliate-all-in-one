import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';
import { PAGES, setThemeCookie, type Theme } from './helpers';

const WCAG = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

const summarize = (violations: Awaited<ReturnType<AxeBuilder['analyze']>>['violations']) =>
  violations.map((v) => ({
    id: v.id,
    impact: v.impact,
    help: v.help,
    nodes: v.nodes.slice(0, 3).map((n) => n.target.join(' ')),
  }));

for (const [path] of PAGES) {
  for (const theme of ['light', 'dark'] as Theme[]) {
    test(`axe: ${path} (${theme}) has no WCAG A/AA violations`, async ({
      page,
      context,
      baseURL,
    }) => {
      await setThemeCookie(context, baseURL!, theme);
      await page.goto(path);
      const wcag = await new AxeBuilder({ page }).withTags(WCAG).analyze();
      expect(summarize(wcag.violations)).toEqual([]);

      const best = await new AxeBuilder({ page }).withTags(['best-practice']).analyze();
      const serious = best.violations.filter((v) => v.impact === 'serious' || v.impact === 'critical');
      expect(summarize(serious)).toEqual([]);
    });
  }
}

test('axe: toasts and form errors are accessible while visible', async ({ page }) => {
  await page.goto('/_ui');
  for (const kind of ['info', 'success', 'warning', 'error'])
    await page.locator(`[data-toast-demo="${kind}"]`).click();
  const results = await new AxeBuilder({ page }).withTags(WCAG).analyze();
  expect(summarize(results.violations)).toEqual([]);
});

test('form fields are wired: labels, descriptions and errors resolve to real elements', async ({
  page,
}) => {
  await page.goto('/_ui');
  const broken = await page.evaluate(() => {
    const missing: string[] = [];
    for (const el of document.querySelectorAll('[aria-describedby]')) {
      for (const id of el.getAttribute('aria-describedby')!.split(' '))
        if (!document.getElementById(id)) missing.push(`${el.id} -> ${id}`);
    }
    for (const el of document.querySelectorAll('input, textarea')) {
      if (!document.querySelector(`label[for="${el.id}"]`)) missing.push(`no label for ${el.id}`);
    }
    return missing;
  });
  expect(broken).toEqual([]);

  const invalid = page.locator('#g-email-bad');
  await expect(invalid).toHaveAttribute('aria-invalid', 'true');
  await expect(page.locator('#g-email-bad-error')).toContainText('Email chưa hợp lệ');
});
