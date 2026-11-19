import { expect, test } from '@playwright/test';
import { PAGES } from './helpers';

// The server sends `default-src 'none'; script-src 'self'; style-src 'self'; …` (no inline, no
// data:). Any inline script/style or third-party request would show up here.
for (const [path] of PAGES) {
  test(`strict CSP: no violations, console errors or external requests on ${path}`, async ({
    page,
    baseURL,
  }) => {
    const problems: string[] = [];
    await page.addInitScript(() => {
      document.addEventListener('securitypolicyviolation', (e) => {
        console.error(`CSP violation: ${e.violatedDirective} ${e.blockedURI}`);
      });
    });
    page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`));
    // The 404 page's own status is logged by Chrome as a console error; subresource failures are
    // caught by the response listener below, so only the document itself is excluded here.
    const own = new URL(path, baseURL!).href;
    page.on('console', (m) => {
      if (m.type() === 'error' && m.location().url !== own) problems.push(`console: ${m.text()}`);
    });
    page.on('requestfailed', (r) => problems.push(`request failed: ${r.url()}`));
    page.on('request', (r) => {
      if (new URL(r.url()).origin !== new URL(baseURL!).origin) problems.push(`external: ${r.url()}`);
    });
    page.on('response', (r) => {
      if (r.request().resourceType() !== 'document' && r.status() >= 400)
        problems.push(`${r.status()}: ${r.url()}`);
    });

    const response = await page.goto(path);
    const csp = response!.headers()['content-security-policy'] ?? '';
    expect(csp).toContain("default-src 'none'");
    expect(csp).not.toMatch(/unsafe-inline|unsafe-eval|data:|\*/);

    // Exercise the script: theme toggle, language links, toasts (gallery only).
    await page.locator('[data-theme-toggle]').click();
    await page.locator('[data-theme-toggle]').click();
    if (path.includes('_ui')) {
      for (const kind of ['info', 'success', 'warning', 'error'])
        await page.locator(`[data-toast-demo="${kind}"]`).click();
      await page.locator('[data-toast-close]').first().click();
    }
    await page.waitForLoadState('networkidle');
    expect(problems).toEqual([]);
  });
}

test('document is free of inline scripts, inline styles and event-handler attributes', async ({
  page,
}) => {
  for (const [path] of PAGES) {
    await page.goto(path);
    const found = await page.evaluate(() => ({
      inlineScripts: [...document.scripts].filter((s) => !s.src).length,
      styleTags: document.querySelectorAll('style').length,
      styleAttrs: document.querySelectorAll('[style]').length,
      handlers: [...document.querySelectorAll('*')].filter((el) =>
        el.getAttributeNames().some((n) => n.startsWith('on')),
      ).length,
    }));
    expect(found, path).toEqual({ inlineScripts: 0, styleTags: 0, styleAttrs: 0, handlers: 0 });
  }
});
