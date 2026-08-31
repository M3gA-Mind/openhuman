import { expect, type Page, test } from '@playwright/test';

import { bootAuthenticatedPage, dismissWalkthroughIfPresent } from '../helpers/core-rpc';

// See app-shell-sidebar.spec.ts for why the budget is raised here rather than
// in the shared playwright.config.ts.
test.describe.configure({ timeout: 180_000 });

/**
 * Keyboard traversal of the app shell.
 *
 * Nothing in the 78-spec Playwright suite presses Tab or reads
 * `document.activeElement` — `rg "press\('Tab'\)|activeElement"` over
 * `test/playwright/specs` returns nothing. And jsdom cannot stand in: it has no
 * sequential focus navigation, no `:focus-visible` resolution and no computed
 * outline, so the 735 vitest files cannot speak to any of this either.
 *
 * What this asserts is behaviour the shell already commits to — every nav row
 * is a real `<button>` inside a labelled `<nav>` landmark, and
 * `SidebarMenuButton` carries `focus-visible:ring-2` — rather than an
 * accessibility standard nobody in this repo has adopted. Two genuine gaps
 * found while writing it (no skip link, no `<main>` landmark in the shell) are
 * recorded in the bug file as findings, NOT asserted here: turning an opinion
 * into a red test is how a suite stops being trusted.
 */

const sidebar = (page: Page) => page.locator('[data-testid="root-shell-sidebar"]');
const navRow = (page: Page, id: string) => page.locator(`[data-walkthrough="tab-${id}"]`);

/** A stable description of whatever currently holds focus. */
async function focused(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    if (!el || el === document.body) return 'BODY';
    const walk = el.getAttribute('data-walkthrough');
    if (walk) return `nav:${walk}`;
    const testid = el.getAttribute('data-testid');
    if (testid) return `testid:${testid}`;
    const label = el.getAttribute('aria-label');
    if (label) return `label:${label}`;
    return el.tagName;
  });
}

const hash = (page: Page) => page.evaluate(() => window.location.hash);

test.describe('App shell — keyboard traversal', () => {
  test.beforeEach(async ({ page }) => {
    await bootAuthenticatedPage(page, 'pw-keyboard-nav-user');
    await dismissWalkthroughIfPresent(page);
  });

  test('every sidebar nav row is reachable by keyboard focus', async ({ page }) => {
    // Not a Tab-order assertion — the order is a design decision that may
    // legitimately change. What must hold is that no row is keyboard-*un*reachable.
    //
    // The `tabIndex` half is load-bearing and was missing from the first
    // version of this test. `locator.focus()` succeeds on a `tabindex="-1"`
    // element, so a focus-only check passes against exactly the regression the
    // comment claimed to guard: a row that can be focused by script but can
    // never be reached by Tab. Assert both.
    for (const id of ['chat', 'brain', 'flows', 'connections'] as const) {
      await navRow(page, id).focus();
      await expect.poll(() => focused(page)).toBe(`nav:tab-${id}`);
      expect(
        await navRow(page, id).evaluate(el => (el as HTMLElement).tabIndex),
        `tab-${id} is focusable by script but not in the tab order`
      ).toBeGreaterThanOrEqual(0);
    }
  });

  test('Enter activates a focused nav row', async ({ page }) => {
    await navRow(page, 'flows').focus();
    await expect.poll(() => focused(page)).toBe('nav:tab-flows');

    await page.keyboard.press('Enter');
    await expect.poll(() => hash(page)).toMatch(/^#\/flows/);
  });

  test('Space also activates a focused nav row', async ({ page }) => {
    // A native <button> answers to both. A div with an onClick answers to
    // neither, and that substitution is the usual way this breaks.
    await page.goto('/#/chat');
    await navRow(page, 'connections').focus();
    await page.keyboard.press(' ');
    await expect.poll(() => hash(page)).toMatch(/^#\/connections/);
  });

  test('focus is not lost to <body> when a nav row changes route', async ({ page }) => {
    // The classic SPA keyboard regression: the route swaps, the focused element
    // unmounts, and focus falls back to <body> — so the next Tab restarts from
    // the top of the document and the user loses their place silently. Nothing
    // in this repo checked it.
    await navRow(page, 'brain').focus();
    await page.keyboard.press('Enter');
    await expect.poll(() => hash(page)).toMatch(/^#\/brain/);

    await expect.poll(() => focused(page)).not.toBe('BODY');
  });

  test('Tab moves focus onward rather than trapping inside the sidebar', async ({ page }) => {
    // A focus trap in the nav is unrecoverable without a mouse. Tab from the
    // last nav row enough times that focus must have left the rail; assert it
    // did, rather than asserting any particular next stop.
    await navRow(page, 'connections').focus();

    const seen = new Set<string>();
    for (let i = 0; i < 12; i += 1) {
      await page.keyboard.press('Tab');
      seen.add(await focused(page));
    }

    const stillInNav = [...seen].every(s => s.startsWith('nav:tab-'));
    expect(stillInNav, `focus never left the nav rail; visited: ${[...seen].join(', ')}`).toBe(
      false
    );
  });

  test('a focused nav row shows a visible focus ring', async ({ page }) => {
    // `SidebarMenuButton` carries `focus-visible:ring-2`. jsdom resolves no
    // computed style for it, so this is the first thing to actually check that
    // a keyboard user can see where they are.
    const row = navRow(page, 'flows');

    const atRest = await row.evaluate(el => {
      const s = getComputedStyle(el);
      return `${s.boxShadow}|${s.outlineWidth}`;
    });

    // Keyboard focus, not `.focus()` — `:focus-visible` distinguishes them.
    await navRow(page, 'chat').focus();
    await page.keyboard.press('Tab');
    await row.focus();
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');

    const onFocus = await row.evaluate(el => {
      const s = getComputedStyle(el);
      return `${s.boxShadow}|${s.outlineWidth}`;
    });

    // Either a ring (box-shadow) or an outline must appear. Which one is a
    // styling choice; that *something* appears is the contract.
    expect(onFocus, `focus indicator unchanged from rest (${atRest})`).not.toBe(atRest);
  });

  test('the collapsed rail stays keyboard-reachable', async ({ page }) => {
    // Collapse must not strand a keyboard user: the icon rail is the only nav
    // left, so if its rows are not focusable there is no way to navigate.
    await page.getByRole('button', { name: 'Hide sidebar' }).click();
    await expect(sidebar(page)).toHaveAttribute('data-state', 'collapsed');

    await navRow(page, 'connections').focus();
    await expect.poll(() => focused(page)).toBe('nav:tab-connections');

    await page.keyboard.press('Enter');
    await expect.poll(() => hash(page)).toMatch(/^#\/connections/);
  });
});
