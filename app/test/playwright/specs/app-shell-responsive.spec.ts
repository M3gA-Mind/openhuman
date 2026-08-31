import { expect, type Page, test } from '@playwright/test';

import { bootAuthenticatedPage, dismissWalkthroughIfPresent } from '../helpers/core-rpc';

// `bootAuthenticatedPage` runs in every `beforeEach` and costs 30-60s against a
// locally-built debug core — the sidebar suite's first test measured 59.1s
// against the config's 60s non-CI budget, and this suite's first two tests blew
// it outright ("Test timeout of 60000ms exceeded while running beforeEach").
// The work is the harness's, not the assertions': raise the ceiling here rather
// than in the shared playwright.config.ts, which is not this worker's to edit.
test.describe.configure({ timeout: 180_000 });

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

  test('PINS CURRENT BEHAVIOUR: the sidebar width has no viewport-relative clamp', async ({
    page,
  }) => {
    // Measured, not assumed. `clampWidth` (`RootShellLayout.tsx:38`) clamps the
    // sidebar against the constants `SIDEBAR_MIN_WIDTH = 188` and
    // `SIDEBAR_MAX_WIDTH = 420` (`components/ui/Sidebar.tsx:48-49`) and never
    // against `window.innerWidth`. So the sidebar keeps its full width however
    // narrow the window gets, and takes a majority of the screen below ~450px.
    //
    // Whether that matters is a product call, and `tauri.conf.json` does not
    // settle it: the window is `resizable: true` with NO `minWidth`, so a user
    // can drag below 414px and the 188px floor then owns half the app.
    //
    // I originally wrote this as `expect(width).toBeLessThan(414 / 2)` — an
    // invariant the product never promised. It failed at 224px. Pinning the
    // real behaviour instead, so that adding a viewport clamp is a deliberate
    // change that updates this test rather than a silent one. See W5 BUG-10.
    await page.setViewportSize({ width: 414, height: 896 });
    await expect.poll(() => page.evaluate(() => window.innerWidth)).toBe(414);

    const count = await sidebar(page).count();
    if (count === 0) return; // a shell that hides it outright is also fine

    const box = await sidebar(page).boundingBox();
    if (box === null || box.width === 0) return; // collapsed away entirely

    // The floor is absolute: never below SIDEBAR_MIN_WIDTH, whatever the window.
    expect(box.width).toBeGreaterThanOrEqual(188);
    // And it currently exceeds half the window at this size. If this ever
    // stops being true, the clamp changed — update the test with the reason.
    expect(box.width).toBeGreaterThan(414 / 2);
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
    // Same right-edge containment the per-viewport cases assert. Added after a
    // mutation exposed the gap: forcing the content surface to a fixed 1600px
    // failed tests 1-4 on exactly this line and left THIS test green, because
    // it only checked the width and the document-overflow probe. The width
    // check passes at 1600 (it is a `>` bound) and the overflow probe never
    // fires, so without this line the test had nothing left that could fail.
    expect(box!.x + box!.width).toBeLessThanOrEqual(1280 + 1);
    expect(await documentOverflowsHorizontally(page)).toBe(false);
  });
});
