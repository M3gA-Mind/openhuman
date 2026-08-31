/**
 * The disabled-toolkits field is a comma-separated free-text list.
 *
 * `ComposioTriagePanel.test.tsx` covers one clean save — `'Gmail, Slack'` →
 * `['gmail', 'slack']`. Real input is not clean: people paste lists with
 * trailing commas, double commas, tabs and stray spacing. Every one of those
 * goes through `split(',').map(trim().toLowerCase()).filter(Boolean)`, and each
 * step is load-bearing:
 *
 *   - drop `filter(Boolean)` and a trailing comma sends `''` to the backend,
 *     which is a toolkit slug matching nothing — or, worse, matching everything
 *     depending on how the server treats an empty entry;
 *   - drop `toLowerCase()` and `Gmail` silently stops matching `gmail`, so the
 *     user's triage exclusion quietly does nothing;
 *   - drop `trim()` and `' slack'` has the same problem with a leading space.
 *
 * All three failures are invisible in the UI: the field still shows what the
 * user typed and the panel still says "Settings saved".
 *
 * The round-trip matters for the same reason — the list is rendered back with
 * `join(', ')`, so loading and re-saving without editing must be a no-op.
 */
import { fireEvent, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { renderWithProviders } from '../../../../test/test-utils';

const hoisted = vi.hoisted(() => ({ getSettings: vi.fn(), updateSettings: vi.fn() }));

vi.mock('../../../../utils/tauriCommands', () => ({
  openhumanGetComposioTriggerSettings: hoisted.getSettings,
  openhumanUpdateComposioTriggerSettings: hoisted.updateSettings,
}));

vi.mock('../../hooks/useSettingsNavigation', () => ({
  useSettingsNavigation: () => ({
    navigateBack: vi.fn(),
    navigateToSettings: vi.fn(),
    breadcrumbs: [],
  }),
}));

async function importPanel() {
  vi.resetModules();
  const mod = await import('../ComposioTriagePanel');
  return mod.default;
}

function settings(toolkits: string[] = [], disabled = false) {
  return { result: { triage_disabled: disabled, triage_disabled_toolkits: toolkits }, logs: [] };
}

async function renderPanel() {
  const Panel = await importPanel();
  renderWithProviders(<Panel />);
  await waitFor(() => expect(screen.queryByText('Loading…')).toBeNull());
}

const field = () => screen.getByPlaceholderText('gmail, slack, ...') as HTMLInputElement;
const save = () => fireEvent.click(screen.getByRole('button', { name: 'Save' }));

/** The toolkit list from the most recent updateSettings call. */
const savedToolkits = () => hoisted.updateSettings.mock.calls.at(-1)?.[0]?.triage_disabled_toolkits;

beforeEach(() => {
  vi.clearAllMocks();
  hoisted.getSettings.mockResolvedValue(settings());
  hoisted.updateSettings.mockResolvedValue({ result: {}, logs: [] });
});

describe('ComposioTriagePanel — the toolkit list is normalised before it is saved', () => {
  test('drops the empty entry a trailing comma leaves behind', async () => {
    await renderPanel();

    fireEvent.change(field(), { target: { value: 'gmail, slack,' } });
    save();

    await waitFor(() => expect(hoisted.updateSettings).toHaveBeenCalled());
    expect(savedToolkits()).toEqual(['gmail', 'slack']);
  });

  test('collapses doubled commas rather than sending blanks', async () => {
    await renderPanel();

    fireEvent.change(field(), { target: { value: 'gmail,,slack' } });
    save();

    await waitFor(() => expect(hoisted.updateSettings).toHaveBeenCalled());
    expect(savedToolkits()).toEqual(['gmail', 'slack']);
  });

  test('lowercases entries so a capitalised slug still matches', async () => {
    await renderPanel();

    fireEvent.change(field(), { target: { value: 'GMAIL, Slack, NoTiOn' } });
    save();

    await waitFor(() => expect(hoisted.updateSettings).toHaveBeenCalled());
    expect(savedToolkits()).toEqual(['gmail', 'slack', 'notion']);
  });

  test('trims surrounding whitespace, including tabs and newlines', async () => {
    await renderPanel();

    fireEvent.change(field(), { target: { value: '  gmail \t,\n slack  ' } });
    save();

    await waitFor(() => expect(hoisted.updateSettings).toHaveBeenCalled());
    expect(savedToolkits()).toEqual(['gmail', 'slack']);
  });

  test('sends an empty list for a field that is only separators and spaces', async () => {
    // Not `['']` — an empty-string toolkit is a slug that matches nothing and
    // has no business reaching the backend.
    await renderPanel();

    fireEvent.change(field(), { target: { value: ' , , ' } });
    save();

    await waitFor(() => expect(hoisted.updateSettings).toHaveBeenCalled());
    expect(savedToolkits()).toEqual([]);
  });
});

describe('ComposioTriagePanel — loading then saving is a no-op', () => {
  test('round-trips a stored list unchanged', async () => {
    // The list is rendered with `join(', ')` and re-parsed with `split(',')`.
    // If those two disagree, simply opening the panel and pressing Save
    // rewrites the user's configuration.
    hoisted.getSettings.mockResolvedValue(settings(['gmail', 'slack', 'notion']));
    await renderPanel();

    expect(field().value).toBe('gmail, slack, notion');
    save();

    await waitFor(() => expect(hoisted.updateSettings).toHaveBeenCalled());
    expect(savedToolkits()).toEqual(['gmail', 'slack', 'notion']);
  });

  test('preserves the loaded triage_disabled flag through an untouched save', async () => {
    hoisted.getSettings.mockResolvedValue(settings(['gmail'], true));
    await renderPanel();

    save();

    await waitFor(() => expect(hoisted.updateSettings).toHaveBeenCalled());
    expect(hoisted.updateSettings.mock.calls.at(-1)?.[0]).toEqual({
      triage_disabled: true,
      triage_disabled_toolkits: ['gmail'],
    });
  });

  test('survives a payload with no toolkit list and saves an empty one', async () => {
    // Pins the OUTCOME, not a particular line: a partial payload must leave an
    // empty field and save an empty list rather than "undefined".
    //
    // Deliberately not described as covering the `?? []` on load — that default
    // is unobservable here. Removing it makes `.join` throw on `undefined`, but
    // the throw lands in the loader's own `.catch`, which only warns, and
    // `loading` is cleared in the `.finally`, so the rendered result is
    // identical. Verified by mutation: deleting `?? []` fails nothing.
    hoisted.getSettings.mockResolvedValue({ result: { triage_disabled: false }, logs: [] });
    await renderPanel();

    expect(field().value).toBe('');
    save();

    await waitFor(() => expect(hoisted.updateSettings).toHaveBeenCalled());
    expect(savedToolkits()).toEqual([]);
  });
});
