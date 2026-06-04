import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  openhumanGetVoiceServerSettings,
  syncNotchVisibility,
  type VoiceServerSettings,
} from '../../utils/tauriCommands';
import { useNotchBootSync } from '../useNotchBootSync';

vi.mock('../../utils/tauriCommands', () => ({
  openhumanGetVoiceServerSettings: vi.fn(),
  syncNotchVisibility: vi.fn(),
}));

const settings = (alwaysOn: boolean) => ({
  result: { always_on_enabled: alwaysOn } as unknown as VoiceServerSettings,
  logs: [],
});

describe('useNotchBootSync', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(syncNotchVisibility).mockResolvedValue(undefined);
  });

  it('does nothing while still bootstrapping', () => {
    renderHook(() => useNotchBootSync(true));
    expect(openhumanGetVoiceServerSettings).not.toHaveBeenCalled();
    expect(syncNotchVisibility).not.toHaveBeenCalled();
  });

  it('syncs the notch to the persisted always-on flag once the core is ready', async () => {
    vi.mocked(openhumanGetVoiceServerSettings).mockResolvedValue(settings(true));
    renderHook(() => useNotchBootSync(false));
    await waitFor(() => expect(syncNotchVisibility).toHaveBeenCalledWith(true));
    expect(openhumanGetVoiceServerSettings).toHaveBeenCalledTimes(1);
  });

  it('only syncs once per boot across re-renders', async () => {
    vi.mocked(openhumanGetVoiceServerSettings).mockResolvedValue(settings(false));
    const { rerender } = renderHook(({ b }) => useNotchBootSync(b), { initialProps: { b: false } });
    await waitFor(() => expect(syncNotchVisibility).toHaveBeenCalledWith(false));
    rerender({ b: false });
    expect(openhumanGetVoiceServerSettings).toHaveBeenCalledTimes(1);
  });

  it('swallows failures (cosmetic) so boot is never blocked', async () => {
    vi.mocked(openhumanGetVoiceServerSettings).mockRejectedValue(new Error('core offline'));
    const debugSpy = vi.spyOn(console, 'debug').mockImplementation(() => {});
    renderHook(() => useNotchBootSync(false));
    await waitFor(() => expect(debugSpy).toHaveBeenCalled());
    expect(syncNotchVisibility).not.toHaveBeenCalled();
    debugSpy.mockRestore();
  });
});
