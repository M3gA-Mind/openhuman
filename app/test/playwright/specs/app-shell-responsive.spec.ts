import { expect, type Page, test } from '@playwright/test';

import { bootAuthenticatedPage, dismissWalkthroughIfPresent } from '../helpers/core-rpc';

/**
 * Narrow-viewport behaviour of the root shell.
 *
 * Nothing in the repo resizes the window. Every existing Playwright spec runs
 * at the default 1280×720 and every vitest suite runs in jsdom, which has no
 * layout engine at all — `getBoundingClientRect()` returns zeroes there, so a
 * layout that overflows or collapses to nothing is undetectable by design.
 *
 * The assertions here are deliberately about containment rather than pixel
 * values: no horizontal overflow of the document, the primary surface keeps a
 * usable width, and the sidebar either adapts or gets out of the way — never
 * eats the content area.
 */

const sidebar = (page: Page) => page.locator('[data-testid="root-shell-sidebar"]');
const content = (page: Page) => page.locator('[data-testid="root-shell-content"]');

async function documentOverflowsHorizontally(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const el = document.documentElement;
    // 1px of slack for sub-pixel rounding on fractional DPR.
    return el.scrollWidth > el.clientWidth + 1;
  });
}

test.describe('App shell — narrow viewports', () => {
  test.beforeEach(async ({ page }) => {
    await bootAuthenticatedPage(page, 'pw-app-shell-responsive-user');
    await dismissWalkthroughIfPresent(page);
  });

  for (const [label, width, height] of [
    ['laptop', 1280, 720],
    ['small laptop', 1024, 768],
    ['tablet portrait', 768, 1024],
    ['large phone', 414, 896],
  ] as const) {
    test(`${label} (${width}×${height}): content stays on screen and the page does not overflow`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height });

      // Let the layout settle rather than sampling mid-transition.
      await expect.poll(() => page.evaluate(() => window.innerWidth)).toBe(width);

      await expect(content(page)).toBeVisible();
      const box = await content(page).boundingBox();
      expect(box).not.toBeNull();

      // The content surface must keep a usable share of the window. The bug
      // this catches is a fixed-width sidebar that does not shrink: the content
      // column gets squeezed toward zero while nothing visibly "breaks".
      expect(box!.width).toBeGreaterThan(width * 0.4);

      // And it must not be pushed off the right edge.
      expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);

      expect(await documentOverflowsHorizontally(page)).toBe(false);
    });
  }

  test('the sidebar never occupies more than half the window at 414px', async ({ page }) => {
    await page.setViewportSize({ width: 414, height: 896 });
    await expect.poll(() => page.evaluate(() => window.innerWidth)).toBe(414);

    const count = await sidebar(page).count();
    if (count === 0) {
      // A shell that hides the sidebar outright at phone width is a valid
      // answer — record it rather than failing.
      expect(count).toBe(0);
      return;
    }

    const box = await sidebar(page).boundingBox();
    if (box === null || box.width === 0) return; // collapsed away entirely
    expect(box.width).toBeLessThan(414 / 2);
  });

  test('resizing back to full width restores the layout', async ({ page }) => {
    // A one-way responsive breakpoint — narrow works, but widening leaves the
    // shell stuck in its compact form — is a real and easy regression.
    await page.setViewportSize({ width: 414, height: 896 });
    await expect.poll(() => page.evaluate(() => window.innerWidth)).toBe(414);

    await page.setViewportSize({ width: 1280, height: 720 });
    await expect.poll(() => page.evaluate(() => window.innerWidth)).toBe(1280);

    await expect(content(page)).toBeVisible();
    const box = await content(page).boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(1280 * 0.4);
    expect(await documentOverflowsHorizontally(page)).toBe(false);
  });
});
