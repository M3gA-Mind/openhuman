import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { useCoreState } from '../../../providers/CoreStateProvider';
import { useScreenIntelligenceSkillStatus } from '../useScreenIntelligenceSkillStatus';

vi.mock('../../../providers/CoreStateProvider', () => ({ useCoreState: vi.fn() }));

type ScreenIntelRuntime = {
  platform_supported: boolean;
  permissions: {
    screen_recording: string;
    accessibility: string;
    input_monitoring: string;
  };
  session: { active: boolean };
  config: { enabled: boolean };
};

function mockSnapshot(screenIntelligence: ScreenIntelRuntime | null): void {
  vi.mocked(useCoreState).mockReturnValue({
    snapshot: { runtime: { screenIntelligence } },
  } as ReturnType<typeof useCoreState>);
}

const grantedPerms = {
  screen_recording: 'granted',
  accessibility: 'granted',
  input_monitoring: 'granted',
} as const;

describe('useScreenIntelligenceSkillStatus', () => {
  it('returns macOS only when platform_supported is false', () => {
    mockSnapshot({
      platform_supported: false,
      permissions: {
        screen_recording: 'unsupported',
        accessibility: 'unsupported',
        input_monitoring: 'unsupported',
      },
      session: { active: false },
      config: { enabled: false },
    });
    const { result } = renderHook(() => useScreenIntelligenceSkillStatus());
    expect(result.current.statusLabel).toBe('macOS only');
    expect(result.current.platformUnsupported).toBe(true);
    expect(result.current.ctaLabel).toBe('Details');
  });

  it('returns Setup when platform is supported but permissions are missing', () => {
    mockSnapshot({
      platform_supported: true,
      permissions: {
        screen_recording: 'unknown',
        accessibility: 'unknown',
        input_monitoring: 'unknown',
      },
      session: { active: false },
      config: { enabled: false },
    });
    const { result } = renderHook(() => useScreenIntelligenceSkillStatus());
    expect(result.current.statusLabel).toBe('Setup');
    expect(result.current.platformUnsupported).toBe(false);
  });

  it('returns Active when session is active', () => {
    mockSnapshot({
      platform_supported: true,
      permissions: { ...grantedPerms },
      session: { active: true },
      config: { enabled: true },
    });
    const { result } = renderHook(() => useScreenIntelligenceSkillStatus());
    expect(result.current.statusLabel).toBe('Active');
    expect(result.current.allPermissionsGranted).toBe(true);
  });
});
