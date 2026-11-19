import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.clock.install(); // control auto-dismiss timers; time still flows normally
  await page.goto('/_ui');
});

test('success toast is announced politely, localized and auto-dismisses', async ({ page }) => {
  await page.locator('[data-toast-demo="success"]').click();
  const toast = page.locator('[data-toast][data-kind="success"]');
  await expect(toast).toBeVisible();
  await expect(toast).toHaveAttribute('role', 'status');
  await expect(toast.locator('[data-toast-title]')).toHaveText('Thành công');
  await expect(toast.locator('[data-toast-message]')).toHaveText('Đã sao chép liên kết.');
  await expect(page.locator('[data-toast-region]')).toHaveAttribute('aria-live', 'polite');

  await page.clock.runFor(7000);
  await expect(toast).toHaveCount(0);
});

test('error toast is an alert, never auto-dismisses and closes on demand', async ({ page }) => {
  await page.locator('[data-toast-demo="error"]').click();
  const toast = page.locator('[data-toast][data-kind="error"]');
  await expect(toast).toHaveAttribute('role', 'alert');
  await page.clock.runFor(60_000);
  await expect(toast).toBeVisible();

  await toast.getByRole('button', { name: 'Đóng thông báo' }).click();
  await expect(toast).toHaveCount(0);
});

test('Escape closes the toast that has focus', async ({ page }) => {
  await page.locator('[data-toast-demo="error"]').click();
  const toast = page.locator('[data-toast][data-kind="error"]');
  await toast.getByRole('button', { name: 'Đóng thông báo' }).focus();
  await page.keyboard.press('Escape');
  await expect(toast).toHaveCount(0);
});

test('toasts stack and each kind shows exactly one icon', async ({ page }) => {
  for (const kind of ['info', 'success', 'warning']) {
    await page.locator(`[data-toast-demo="${kind}"]`).click();
  }
  const toasts = page.locator('[data-toast]');
  await expect(toasts).toHaveCount(3);
  for (const toast of await toasts.all()) {
    await expect(toast.locator('.toast-icon:visible')).toHaveCount(1);
  }
});

test('toast text is inserted as text, never as markup', async ({ page }) => {
  await page.evaluate(() => {
    (window as unknown as { AAO: { toast: (o: object) => void } }).AAO.toast({
      kind: 'info',
      message: '<img src=x onerror=alert(1)>',
    });
  });
  const message = page.locator('[data-toast] [data-toast-message]');
  await expect(message).toHaveText('<img src=x onerror=alert(1)>');
  await expect(page.locator('[data-toast] img')).toHaveCount(0);
});

test('English toasts use English titles', async ({ page }) => {
  await page.goto('/en/_ui');
  await page.locator('[data-toast-demo="warning"]').click();
  await expect(page.locator('[data-toast][data-kind="warning"] [data-toast-title]')).toHaveText(
    'Heads up',
  );
});
