/**
 * The custom font-size field's reject and clamp branches.
 *
 * `AppearancePanel.test.tsx` covers the happy paths — the four presets, the
 * slider, and one blur commit that clamps 99 down to the 28px maximum. What it
 * does not touch is `commitCustomFontSize`'s `else`: the branch that runs when
 * the draft does not parse as a number.
 *
 * That branch matters because the field is a free-text draft (`pxDraft`) that is
 * deliberately NOT pushed to the store on every keystroke. If a non-numeric
 * commit fell through to `dispatch(setCustomFontSizePx(NaN))`, the clamp is
 * `Math.min(28, Math.max(12, Math.round(NaN)))` — which is `NaN`, not a bound —
 * and the app's root font size becomes `NaNpx`. The `else` is what stops that,
 * and nothing asserted it.
 *
 * The lower clamp is covered here too; the existing spec only exercises the
 * upper one, and a one-sided clamp is a common way to ship half a fix.
 */
import { fireEvent, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '../../../test/test-utils';
import AppearancePanel from './AppearancePanel';

vi.mock('../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (key: string) => key }) }));
vi.mock('../hooks/useSettingsNavigation', () => ({
  useSettingsNavigation: () => ({ breadcrumbs: [], navigateBack: vi.fn() }),
}));

function renderPanel(
  fontSize: 'small' | 'medium' | 'large' | 'xlarge' = 'medium',
  customFontSizePx: number | null = null
) {
  return renderWithProviders(<AppearancePanel />, {
    preloadedState: {
      theme: {
        mode: 'system',
        tabBarLabels: 'hover',
        fontSize,
        customFontSizePx,
        agentMessageViewMode: 'bubbles',
      },
    },
  });
}

const field = (getByTestId: (id: string) => HTMLElement) =>
  within(getByTestId('font-size-custom-number')).getByRole('spinbutton') as HTMLInputElement;

describe('AppearancePanel — a custom font size that does not parse is rejected', () => {
  it('does not write to the store when the draft is not a number', () => {
    const { getByTestId, store } = renderPanel('medium', 20);
    const input = field(getByTestId);

    fireEvent.change(input, { target: { value: 'abc' } });
    fireEvent.blur(input);

    // Unchanged — not NaN, and not silently coerced to a bound either.
    expect(store.getState().theme.customFontSizePx).toBe(20);
  });

  it('reverts the visible draft to the effective size after a rejected commit', () => {
    // Leaving "abc" in the box after the commit was thrown away tells the user
    // their value took effect when it did not.
    const { getByTestId } = renderPanel('medium', 20);
    const input = field(getByTestId);

    fireEvent.change(input, { target: { value: 'abc' } });
    fireEvent.blur(input);

    expect(input.value).toBe('20');
  });

  it('rejects an emptied field rather than storing a blank', () => {
    // `Number.parseInt('', 10)` is NaN — the same branch, via the most likely
    // user action: select-all then tab away.
    const { getByTestId, store } = renderPanel('medium', 20);
    const input = field(getByTestId);

    fireEvent.change(input, { target: { value: '' } });
    fireEvent.blur(input);

    expect(store.getState().theme.customFontSizePx).toBe(20);
    expect(input.value).toBe('20');
  });
});

describe('AppearancePanel — the custom font size is clamped at both ends', () => {
  it('clamps below the minimum up to 12px', () => {
    // The existing spec clamps 99 -> 28. This is the other half: a one-sided
    // clamp still ships an unreadable 1px UI.
    const { getByTestId, store } = renderPanel('medium');
    const input = field(getByTestId);

    fireEvent.change(input, { target: { value: '1' } });
    fireEvent.blur(input);

    expect(store.getState().theme.customFontSizePx).toBe(12);
  });

  it('clamps a negative value up to the minimum', () => {
    const { getByTestId, store } = renderPanel('medium');
    const input = field(getByTestId);

    fireEvent.change(input, { target: { value: '-5' } });
    fireEvent.blur(input);

    expect(store.getState().theme.customFontSizePx).toBe(12);
  });

  it('accepts a value inside the range unchanged', () => {
    const { getByTestId, store } = renderPanel('medium');
    const input = field(getByTestId);

    fireEvent.change(input, { target: { value: '19' } });
    fireEvent.blur(input);

    expect(store.getState().theme.customFontSizePx).toBe(19);
  });
});

describe('AppearancePanel — the draft follows changes made elsewhere', () => {
  it('re-syncs the number field when a preset is chosen', () => {
    // `pxDraft` is local state seeded once from the store. The render-phase
    // re-sync (`if (effectiveFontSizePx !== syncedPx)`) is what keeps it honest
    // when the size changes from the presets or the slider instead of the box.
    // Without it the field keeps showing the old number while the UI resizes.
    const { getByTestId, getByRole } = renderPanel('medium', 20);
    expect(field(getByTestId).value).toBe('20');

    const group = getByRole('radiogroup', { name: 'settings.appearance.fontSizeAria' });
    fireEvent.click(within(group).getByRole('radio', { name: /fontSizeSmall/ }));

    expect(field(getByTestId).value).not.toBe('20');
  });
});
