import { expect, test } from '@playwright/test';

test('create a project and see it listed', async ({ page }) => {
  await page.goto('/');
  await page.getByPlaceholder('Project name').fill('CI Mix');
  await page.getByRole('button', { name: /create/i }).click();
  await expect(page.getByText('CI Mix')).toBeVisible();
});

test('import shows a track with waveform', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /import/i }).click();
  await expect(page.getByText('E2E Track')).toBeVisible();
  await expect(page.getByTestId('waveform')).toBeVisible();
});