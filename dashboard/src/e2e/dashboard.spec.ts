import { test, expect } from '@playwright/test';

test.describe('GKS Insight Dashboard E2E Audit', () => {
  test.beforeEach(async ({ page }) => {
    // Assuming the dashboard runs on http://localhost:5175
    await page.goto('http://localhost:5175');
  });

  test('should display real system data from backend', async ({ page }) => {
    // Wait for the main layout to appear
    await page.waitForSelector('h1', { timeout: 10000 });

    // Stricter check: Peer ID should NOT be "Connecting..." after a few seconds
    const peerId = page.locator('p.text-accent-blue');
    await expect(peerId).not.toHaveText('Connecting...', { timeout: 15000 });

    // Metrics should not be placeholders
    const metrics = page.locator('p.text-white');
    const count = await metrics.count();
    for (let i = 0; i < count; i++) {
      await expect(metrics.nth(i)).not.toHaveText('---', { timeout: 10000 });
    }
  });

  test('should have a functional refresh button that updates data', async ({ page }) => {
    const refreshBtn = page.getByRole('button', { name: 'Refresh' });
    await expect(refreshBtn).toBeVisible();

    // Click refresh and ensure it doesn't break the data display
    await refreshBtn.click();
    await expect(page.locator('p.text-accent-blue')).not.toHaveText('Connecting...');
  });

  test('should display swarm section', async ({ page }) => {
    await expect(page.getByText('Swarm Peers')).toBeVisible();
  });
});
