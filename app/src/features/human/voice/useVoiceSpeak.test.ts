import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const hoisted = vi.hoisted(() => ({
  onMock: vi.fn<(event: string, cb: (...args: unknown[]) => void) => void>(),
  offMock: vi.fn(),
  synthMock: vi.fn(),
  playMock: vi.fn(),
  stopMock: vi.fn(),
}));

vi.mock('../../../services/socketService', () => ({
  socketService: { on: hoisted.onMock, off: hoisted.offMock },
}));
vi.mock('./ttsClient', () => ({ synthesizeSpeech: hoisted.synthMock }));
vi.mock('./audioPlayer', () => ({
  playBase64Audio: hoisted.playMock,
  swallowAudioStop: vi.fn(),
}));

import { useVoiceSpeak } from './useVoiceSpeak';

/** Grab the `voice:speak` handler the hook registered with socketService. */
function speakHandler(): (...args: unknown[]) => void {
  const call = hoisted.onMock.mock.calls.find(([event]) => event === 'voice:speak');
  if (!call) throw new Error('useVoiceSpeak did not subscribe to voice:speak');
  return call[1];
}

describe('useVoiceSpeak', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hoisted.synthMock.mockResolvedValue({
      audio_base64: 'AAA=',
      audio_mime: 'audio/mpeg',
      visemes: [],
    });
    hoisted.playMock.mockResolvedValue({ ended: Promise.resolve(), stop: hoisted.stopMock });
  });

  it('synthesizes and plays the spoken prompt on voice:speak', async () => {
    renderHook(() => useVoiceSpeak());
    speakHandler()({ text: 'Post to Slack. Say yes to confirm.', source: 'approval' });

    await vi.waitFor(() => expect(hoisted.playMock).toHaveBeenCalledTimes(1));
    expect(hoisted.synthMock).toHaveBeenCalledWith('Post to Slack. Say yes to confirm.');
    expect(hoisted.playMock).toHaveBeenCalledWith('AAA=', 'audio/mpeg', expect.any(Object));
  });

  it('ignores an empty/whitespace prompt without synthesizing', async () => {
    renderHook(() => useVoiceSpeak());
    speakHandler()({ text: '   ' });
    await Promise.resolve();
    expect(hoisted.synthMock).not.toHaveBeenCalled();
    expect(hoisted.playMock).not.toHaveBeenCalled();
  });

  it('ignores a malformed payload', async () => {
    renderHook(() => useVoiceSpeak());
    speakHandler()({ notText: true });
    await Promise.resolve();
    expect(hoisted.synthMock).not.toHaveBeenCalled();
  });

  it('unsubscribes and stops playback on unmount', () => {
    const { unmount } = renderHook(() => useVoiceSpeak());
    unmount();
    expect(hoisted.offMock).toHaveBeenCalledWith('voice:speak', expect.any(Function));
  });
});
