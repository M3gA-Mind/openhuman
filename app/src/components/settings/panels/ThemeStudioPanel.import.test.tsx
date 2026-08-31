/**
 * Theme import — the rejection paths.
 *
 * `ThemeStudioPanel.test.tsx` covers the gallery, the auto-fork on colour edit,
 * and one successful import that preserves gradient/backdrop. Every failure path
 * is open, and this is the one surface in the panel that parses text the user
 * pasted from somewhere else.
 *
 * What matters here is that a bad paste is *reported and survivable*: the error
 * is shown, no half-built theme is written to the store, and the text stays in
 * the box so it can be corrected. Silently swallowing a malformed theme — or
 * worse, storing one and letting it be activated — is how you get an unstyled
 * app with no way back.
 *
 * i18n is deliberately NOT mocked, matching the existing spec: `t()` is called
 * with literal fallbacks here (`t('settings.theme.importError', 'Could not
 * parse…')`), so the rendered copy is what a user sees.
 */
import { fireEvent, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { renderWithProviders } from '../../../test/test-utils';
import ThemeStudioPanel from './ThemeStudioPanel';

const themeState = {
  mode: 'system',
  tabBarLabels: 'hover',
  fontSize: 'medium',
  activeThemeId: 'system',
  customThemes: [],
};

function renderPanel() {
  return renderWithProviders(<ThemeStudioPanel />, {
    preloadedState: { theme: themeState },
    initialEntries: ['/settings/theme'],
  });
}

/**
 * Selected by its aria-label, not by role: once a custom theme exists the panel
 * also renders the read-only export box ("Copy JSON"), so a bare
 * `getByRole('textbox')` is ambiguous exactly on the successful-import paths.
 */
const importBox = () => screen.getByLabelText(/import theme/i) as HTMLTextAreaElement;
const importButton = () => screen.getByRole('button', { name: /^Import$/i });

function paste(json: string) {
  fireEvent.change(importBox(), { target: { value: json } });
}

describe('ThemeStudioPanel — a bad paste is reported, not swallowed', () => {
  it('reports malformed JSON and stores nothing', () => {
    const { store } = renderPanel();

    paste('{ not json');
    fireEvent.click(importButton());

    expect(screen.getByText(/could not parse/i)).toBeInTheDocument();
    expect(store.getState().theme.customThemes).toHaveLength(0);
  });

  it('keeps the pasted text after a failure so it can be corrected', () => {
    // Clearing the box on failure throws away what the user was trying to fix.
    const { store } = renderPanel();

    paste('{ not json');
    fireEvent.click(importButton());

    expect(importBox().value).toBe('{ not json');
    expect(store.getState().theme.customThemes).toHaveLength(0);
  });

  it('rejects valid JSON that carries no colors', () => {
    const { store } = renderPanel();

    paste(JSON.stringify({ name: 'No colours', isDark: false }));
    fireEvent.click(importButton());

    expect(screen.getByText(/could not parse/i)).toBeInTheDocument();
    expect(store.getState().theme.customThemes).toHaveLength(0);
  });

  it('rejects a colors field that is not an object', () => {
    const { store } = renderPanel();

    paste(JSON.stringify({ name: 'Stringy', colors: 'not-an-object' }));
    fireEvent.click(importButton());

    expect(screen.getByText(/could not parse/i)).toBeInTheDocument();
    expect(store.getState().theme.customThemes).toHaveLength(0);
  });

  it('rejects a bare JSON array', () => {
    const { store } = renderPanel();

    paste('[]');
    fireEvent.click(importButton());

    expect(screen.getByText(/could not parse/i)).toBeInTheDocument();
    expect(store.getState().theme.customThemes).toHaveLength(0);
  });

  it('clears a previous error when a later import succeeds', () => {
    // `handleImport` resets the error first. Without that the box keeps
    // accusing the user of a paste they already fixed.
    const { store } = renderPanel();

    paste('{ not json');
    fireEvent.click(importButton());
    expect(screen.getByText(/could not parse/i)).toBeInTheDocument();

    paste(JSON.stringify({ name: 'Good', colors: { 'surface-canvas': '255 255 255' } }));
    fireEvent.click(importButton());

    expect(screen.queryByText(/could not parse/i)).not.toBeInTheDocument();
    expect(store.getState().theme.customThemes).toHaveLength(1);
  });

  it('cannot be submitted while the box is blank or whitespace', () => {
    renderPanel();

    expect(importButton()).toBeDisabled();

    paste('   ');
    expect(importButton()).toBeDisabled();
  });
});

describe('ThemeStudioPanel — a good paste lands cleanly', () => {
  it('stores the theme and empties the box', () => {
    const { store } = renderPanel();

    paste(JSON.stringify({ name: 'Midnight', isDark: true, colors: { content: '0 0 0' } }));
    fireEvent.click(importButton());

    const themes = store.getState().theme.customThemes;
    expect(themes).toHaveLength(1);
    expect(themes[0].name).toBe('Midnight');
    expect(themes[0].isDark).toBe(true);
    expect(themes[0].builtIn).toBe(false);
    expect(importBox().value).toBe('');
  });

  it('names an unnamed theme rather than storing a blank', () => {
    const { store } = renderPanel();

    paste(JSON.stringify({ colors: { content: '0 0 0' } }));
    fireEvent.click(importButton());

    expect(store.getState().theme.customThemes[0].name).toBeTruthy();
  });

  it('drops a backdrop whose kind is not one this build renders', () => {
    // An unknown `kind` reaching the renderer is a blank backdrop with no
    // explanation; `importedBackdrop` is meant to strip it at the door.
    const { store } = renderPanel();

    paste(
      JSON.stringify({
        name: 'Odd backdrop',
        colors: { content: '0 0 0' },
        backdrop: { kind: 'hologram', imageUrl: 'https://example.test/x.png' },
      })
    );
    fireEvent.click(importButton());

    expect(store.getState().theme.customThemes[0].backdrop).toBeUndefined();
  });

  it('keeps a backdrop whose kind is supported', () => {
    const { store } = renderPanel();

    paste(
      JSON.stringify({
        name: 'Mesh backdrop',
        colors: { content: '0 0 0' },
        backdrop: { kind: 'mesh' },
      })
    );
    fireEvent.click(importButton());

    expect(store.getState().theme.customThemes[0].backdrop).toMatchObject({ kind: 'mesh' });
  });
});
