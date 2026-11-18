import type { BrowserContext, Page } from '@playwright/test';

/** Every route the V0.1 server renders: [path, expected language]. */
export const PAGES: ReadonlyArray<readonly [string, 'vi' | 'en']> = [
  ['/', 'vi'],
  ['/en', 'en'],
  ['/_ui', 'vi'],
  ['/en/_ui', 'en'],
  ['/khong-co-trang', 'vi'],
];

export type Theme = 'light' | 'dark';

export async function setThemeCookie(context: BrowserContext, baseURL: string, theme: Theme) {
  await context.addCookies([{ name: 'aao_theme', value: theme, url: baseURL }]);
}

export async function cookieValue(context: BrowserContext, name: string) {
  return (await context.cookies()).find((c) => c.name === name)?.value;
}

/** Computed page background: the cheapest observable that proves a theme is applied. */
export function pageBackground(page: Page) {
  return page.evaluate(() => getComputedStyle(document.body).backgroundColor);
}

export const LIGHT_BG = 'rgb(255, 255, 255)'; // --color-bg, light
export const DARK_BG = 'rgb(11, 18, 32)'; // --color-bg, dark
