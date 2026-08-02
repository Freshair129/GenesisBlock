import { expect, test } from '@playwright/test';

test('connects to the explicit fixture workspace and renders a bounded graph', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: /One place to see/ })).toBeVisible();
  await expect(page.getByText('Backend-enforced read only')).toBeVisible();
  await page.getByRole('button', { name: /Open fixture workspace/ }).click();

  await expect(page.getByRole('heading', { name: 'Graph' })).toBeVisible();
  await expect(page.getByText('MOCK / READ ONLY', { exact: true })).toBeVisible();
  await expect(page.getByLabel('Fixture knowledge graph')).toBeVisible();
  await expect(page.getByText('240 / 1000 node budget')).toBeVisible();
});

test('runs a registered relational query without exposing raw SQLite', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /Open fixture workspace/ }).click();
  await page.getByRole('button', { name: 'Data workspace' }).click();

  await expect(page.getByText('knowledge', { exact: true })).toBeVisible();
  await expect(page.getByText('6 tables / 1 named queries')).toBeVisible();
  await page.getByRole('combobox').selectOption('knowledge:recent_evidence');
  await page.getByRole('button', { name: /Run named query/ }).click();
  await expect(page.getByText(/fixture-only/)).toBeVisible();
});
