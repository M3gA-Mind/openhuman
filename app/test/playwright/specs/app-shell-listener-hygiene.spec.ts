import { expect, type Page, test } from '@playwright/test';

import { bootAuthenticatedPage, dismissWalkthroughIfPresent } from '../helpers/core-rpc';

// See app-shell-sidebar.spec.ts for why the budget is raised here rather than
// in the shared playwright.config.ts.
test.describe.configure({ timeout: 180_000 });

/**
 * Window listener hygiene around the sidebar resize drag — W5 BUG-12.
 *
 * The drag attaches four window listeners and detaches three. It adds
 * `'blur'` but removes `'blur-sm'`:
 *
 *   components/ui/Sidebar.tsx:291   window.addEventListener('blur', detach);
 *   components/ui/Sidebar.tsx:283   window.removeEventListener('blur-sm', detach);
 *
 * and `RootShellLayout.tsx` repeats it at :202 / :185. `'blur-sm'` is not an
 * event — nothing is ever registered under that name — so the removal is a
 * no-op and the real listener survives every drag. `rg "'blur-sm'" app/src`
 * returns exactly these two hits, both inside `removeEventListener` and neither
 * in a `className`, which is what points at a Tailwind v4 `blur` -> `blur-sm`
 * class-rename codemod reaching into string arguments.
 *
 * The leak is functionally quiet — each stale handler still runs a teardown
 * that is idempotent — so nothing fails today. What accumulates is window
 * listeners, on a desktop window that is not reloaded for days.
 *
 * This spec instruments `add`/`removeEventListener` before the app boots and
 * counts. It is written to FAIL on `main` today: it describes the contract the
 * code intends, not the behaviour it has. See the note on the failing
 * assertion.
 */

/** Wrap window listener registration so the test can count what is live. */
async function instrumentListeners(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const counts = new Map<string, number>();
    (window as unknown as { __listenerCounts: Map<string, number> }).__listenerCounts = counts;

    const add = window.addEventListener.bind(window);
    const remove = window.removeEventListener.bind(window);

    window.addEventListener = function (type: string, ...rest: unknown[]) {
      counts.set(type, (counts.get(type) ?? 0) + 1);
      return (add as (...a: unknown[]) => void)(type, ...rest);
    } as typeof window.addEventListener;

    window.removeEventListener = function (type: string, ...rest: unknown[]) {
      // Only decrement for a type we have seen added, so a stray removal
      // cannot drive the count negative and mask a leak.
      if ((counts.get(type) ?? 0) > 0) counts.set(type, (counts.get(type) ?? 0) - 1);
      return (remove as (...a: unknown[]) => void)(type, ...rest);
    } as typeof window.removeEventListener;
  });
}

const countOf = (page: Page, type: string) =>
  page.evaluate(
    t =>
      (window as unknown as { __listenerCounts: Map<string, number> }).__listenerCounts.get(t) ?? 0,
    type
  );

const sidebar = (page: Page) => page.locator('[data-testid="root-shell-sidebar"]');

/** Drag the rail from the half of its hit area that receives events (BUG-11). */
async function dragRail(page: Page, dx: number): Promise<void> {
  const box = await sidebar(page).boundingBox();
  expect(box).not.toBeNull();
  const x = box!.x + box!.width - 2;
  const y = box!.y + box!.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx, y, { steps: 8 });
  await page.mouse.up();
}

test.describe('App shell — the resize drag cleans up after itself (BUG-12)', () => {
  test.beforeEach(async ({ page }) => {
    await instrumentListeners(page);
    await bootAuthenticatedPage(page, 'pw-listener-hygiene-user');
    await dismissWalkthroughIfPresent(page);
  });

  test('pointer listeners are balanced across a drag', async ({ page }) => {
    // The three correctly-paired ones, as a control: if these did not balance,
    // the instrumentation itself would be suspect and the blur result below
    // would mean nothing.
    const before = {
      pointermove: await countOf(page, 'pointermove'),
      pointerup: await countOf(page, 'pointerup'),
      pointercancel: await countOf(page, 'pointercancel'),
    };

    await dragRail(page, 40);

    await expect.poll(() => countOf(page, 'pointermove')).toBe(before.pointermove);
    await expect.poll(() => countOf(page, 'pointerup')).toBe(before.pointerup);
    await expect.poll(() => countOf(page, 'pointercancel')).toBe(before.pointercancel);
  });

  test('blur listeners are balanced across a drag — KNOWN FAILING, W5 BUG-12', async ({ page }) => {
    // `test.fail()`: this asserts the contract the code intends, and that
    // contract is currently broken, so the expected outcome on `main` is a
    // failure. Playwright inverts the result — the spec is GREEN today and goes
    // RED the moment someone fixes the bug, which is the prompt to delete this
    // annotation and the note below.
    //
    // A plain red test would just be broken CI. A skipped one would say
    // nothing. This is the only form that both reports the defect and cleans
    // itself up.
    //
    // Measured on main: baseline 3, +2 per drag (one per leaking site), so
    // three drags end at 9. Verified fix — replace the two `'blur-sm'` string
    // literals with `'blur'` in Sidebar.tsx:283 and RootShellLayout.tsx:185 —
    // makes this pass and leaves the rest of the suite green.
    test.fail();

    const before = await countOf(page, 'blur');

    await dragRail(page, 40);
    await dragRail(page, -40);
    await dragRail(page, 25);

    expect(await countOf(page, 'blur')).toBe(before);
  });
});
