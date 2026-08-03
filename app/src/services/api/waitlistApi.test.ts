import { beforeEach, describe, expect, it, vi } from 'vitest';

const mockPost = vi.fn();

vi.mock('../apiClient', () => ({
  apiClient: {
    post: (...args: unknown[]) => mockPost(...args),
  },
}));

describe('confirmWaitlistDownload', () => {
  beforeEach(() => {
    mockPost.mockReset();
  });

  it('posts the token to the confirm endpoint', async () => {
    mockPost.mockResolvedValueOnce({ success: true, data: {} });

    const { confirmWaitlistDownload } = await import('./waitlistApi');
    await confirmWaitlistDownload('tok_abc123');

    const [endpoint, body] = mockPost.mock.calls[0];
    expect(endpoint).toBe('/waitlist/tasks/download/confirm');
    expect(body).toEqual({ token: 'tok_abc123' });
  });

  it('sends no session bearer — the download token is the credential', async () => {
    mockPost.mockResolvedValueOnce({ success: true, data: {} });

    const { confirmWaitlistDownload } = await import('./waitlistApi');
    await confirmWaitlistDownload('tok_abc123');

    const options = mockPost.mock.calls[0][2] as { requireAuth?: boolean; timeout?: number };
    expect(options.requireAuth).toBe(false);
  });

  it('bounds the request so it cannot hold up app startup', async () => {
    mockPost.mockResolvedValueOnce({ success: true, data: {} });

    const { confirmWaitlistDownload } = await import('./waitlistApi');
    await confirmWaitlistDownload('tok_abc123');

    const options = mockPost.mock.calls[0][2] as { timeout?: number };
    expect(options.timeout).toBeGreaterThan(0);
    expect(options.timeout).toBeLessThan(120_000);
  });

  it('propagates failures so the caller decides how to degrade', async () => {
    mockPost.mockRejectedValueOnce({ success: false, error: 'Waitlist entry not found' });

    const { confirmWaitlistDownload } = await import('./waitlistApi');
    await expect(confirmWaitlistDownload('tok_missing')).rejects.toEqual({
      success: false,
      error: 'Waitlist entry not found',
    });
  });
});
