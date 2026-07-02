/**
 * Tests for CoreConnectionPanel (GH-4396) — the first-class Settings surface
 * that promotes cloud-mode remote-core config and adds a live status
 * indicator. Covers: live status rendering per mode, the remote toggle
 * revealing the URL/token form, and the save flow persisting + dispatching +
 * restarting.
 */
import { fireEvent, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { renderWithProviders } from '../../../../test/test-utils';

const hoisted = vi.hoisted(() => ({
  testCoreRpcConnection: vi.fn(),
  clearCoreRpcUrlCache: vi.fn(),
  clearCoreRpcTokenCache: vi.fn(),
  restartApp: vi.fn(),
}));

vi.mock('../../../../services/coreRpcClient', () => ({
  testCoreRpcConnection: hoisted.testCoreRpcConnection,
  clearCoreRpcUrlCache: hoisted.clearCoreRpcUrlCache,
  clearCoreRpcTokenCache: hoisted.clearCoreRpcTokenCache,
}));

vi.mock('../../../../utils/tauriCommands/core', () => ({
  restartApp: hoisted.restartApp,
}));

function okResponse() {
  return { ok: true, status: 200, json: async () => ({ jsonrpc: '2.0', id: 1, result: {} }) };
}

async function importPanel() {
  const mod = await import('../CoreConnectionPanel');
  return mod.default;
}

describe('CoreConnectionPanel', () => {
  beforeEach(() => {
    vi.resetModules();
    hoisted.testCoreRpcConnection.mockReset();
    hoisted.clearCoreRpcUrlCache.mockReset();
    hoisted.clearCoreRpcTokenCache.mockReset();
    hoisted.restartApp.mockReset();
    hoisted.restartApp.mockResolvedValue(undefined);
    localStorage.clear();
  });

  test('local mode shows the local connected status once the live check passes', async () => {
    hoisted.testCoreRpcConnection.mockResolvedValue(okResponse());
    const Panel = await importPanel();
    renderWithProviders(<Panel />, { preloadedState: { coreMode: { mode: { kind: 'local' } } } });

    await waitFor(() => expect(screen.getByText('Connected to local core')).toBeInTheDocument());
    // Remote toggle is off in local mode → no URL field.
    expect(screen.queryByLabelText(/Runtime URL/i)).not.toBeInTheDocument();
  });

  test('cloud mode surfaces the remote URL and remote connected status', async () => {
    hoisted.testCoreRpcConnection.mockResolvedValue(okResponse());
    const Panel = await importPanel();
    renderWithProviders(<Panel />, {
      preloadedState: {
        coreMode: {
          mode: { kind: 'cloud', url: 'https://core.example.com/rpc', token: 'tok-123456' },
        },
      },
    });

    await waitFor(() =>
      expect(screen.getByText('Connected to remote core')).toBeInTheDocument()
    );
    // Toggle on → the URL field is pre-filled with the persisted value.
    expect(screen.getByDisplayValue('https://core.example.com/rpc')).toBeInTheDocument();
  });

  test('unreachable core surfaces the failure status', async () => {
    hoisted.testCoreRpcConnection.mockRejectedValue(new Error('boom'));
    const Panel = await importPanel();
    renderWithProviders(<Panel />, { preloadedState: { coreMode: { mode: { kind: 'local' } } } });

    await waitFor(() =>
      expect(screen.getByText(/Cannot reach the core/i)).toBeInTheDocument()
    );
  });

  test('switching to remote core persists, dispatches, and restarts', async () => {
    hoisted.testCoreRpcConnection.mockResolvedValue(okResponse());
    const Panel = await importPanel();
    const { store } = renderWithProviders(<Panel />, {
      preloadedState: { coreMode: { mode: { kind: 'local' } } },
    });

    await waitFor(() => expect(screen.getByText('Connected to local core')).toBeInTheDocument());

    // Flip the remote toggle on to reveal the form.
    fireEvent.click(screen.getByTestId('core-use-remote-toggle'));

    fireEvent.change(screen.getByLabelText(/Runtime URL/i), {
      target: { value: 'https://core.example.com/rpc' },
    });
    fireEvent.change(screen.getByLabelText(/Auth Token/i), {
      target: { value: 'remote-token-xyz' },
    });

    fireEvent.click(screen.getByTestId('core-save-btn'));

    await waitFor(() => expect(hoisted.restartApp).toHaveBeenCalledTimes(1));

    // Redux is now in cloud mode with the typed URL + token.
    const mode = store.getState().coreMode.mode as {
      kind: string;
      url?: string;
      token?: string;
    };
    expect(mode.kind).toBe('cloud');
    expect(mode.url).toBe('https://core.example.com/rpc');
    expect(mode.token).toBe('remote-token-xyz');

    // Persisted synchronously to localStorage (mirrors the cloud-mode picker).
    expect(localStorage.getItem('openhuman_core_mode')).toBe('cloud');
    expect(localStorage.getItem('openhuman_core_rpc_url')).toBe('https://core.example.com/rpc');
    expect(localStorage.getItem('openhuman_core_rpc_token')).toBe('remote-token-xyz');

    // Caches cleared so the new endpoint takes effect on restart.
    expect(hoisted.clearCoreRpcUrlCache).toHaveBeenCalled();
    expect(hoisted.clearCoreRpcTokenCache).toHaveBeenCalled();
  });
});
