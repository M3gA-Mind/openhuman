/**
 * Mascot voice preview — the stale-response race and its error path.
 *
 * `MascotPanel.test.tsx` is thorough (38 tests) and its harness already stubs
 * `synthesizeSpeech`, but nothing calls the preview button: its "preview" tests
 * are all about the *visual* mascot (`manifest-mascot-preview-*`,
 * `custom-gif-mascot`). `onVoicePreview` — the one async handler in the panel
 * that starts audio — is unexercised.
 *
 * It carries the same shape of guard the avatar upload has, and that one IS
 * tested ("discards an upload superseded by Reset while the file was still
 * reading"). `previewRequestIdRef` is bumped on every click and on unmount, and
 * checked three times — after the await, in the catch, and in the finally. Get
 * any of those wrong and a preview the user has already moved on from starts
 * talking over the new one, or leaves the button stuck on "Previewing…".
 *
 * `window.Audio` is stubbed: jsdom has no media stack, so the real constructor
 * would make `play()` reject and turn every success case into an error case.
 */
import { configureStore } from '@reduxjs/toolkit';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { Provider } from 'react-redux';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import mascotReducer from '../../../../store/mascotSlice';
import MascotPanel from '../MascotPanel';

const { useMascotManifestMock, mockSynthesizeSpeech } = vi.hoisted(() => ({
  useMascotManifestMock: vi.fn(),
  mockSynthesizeSpeech: vi.fn(),
}));

vi.mock('../../../../features/human/Mascot/manifest/useMascotManifest', () => ({
  useMascotManifest: () => useMascotManifestMock(),
}));
vi.mock('../../../../features/human/voice/ttsClient', () => ({
  synthesizeSpeech: (...args: unknown[]) => mockSynthesizeSpeech(...args),
}));
vi.mock('../../../../features/human/Mascot', async importOriginal => {
  const actual = await importOriginal<typeof import('../../../../features/human/Mascot')>();
  return {
    ...actual,
    RiveMascot: () => <div data-testid="rive-mascot-preview" />,
    ManifestRiveMascot: () => <div data-testid="manifest-mascot-preview" />,
    CustomGifMascot: ({ src }: { src: string }) => (
      <img data-testid="custom-gif-mascot" src={src} alt="" />
    ),
  };
});
vi.mock('../../hooks/useSettingsNavigation', () => ({
  useSettingsNavigation: () => ({ navigateBack: vi.fn(), breadcrumbs: [{ label: 'Settings' }] }),
}));

/**
 * Every Audio the panel constructs, so overlap and teardown are observable.
 *
 * A class, not `vi.fn(src => ({...}))`: the panel calls `new window.Audio(src)`,
 * and an arrow function is not a constructor — `new` on one throws a TypeError
 * that the handler catches and renders as "Voice preview failed: (src) => {…".
 * That looked exactly like a product bug in the first draft of this file.
 */
const audios: FakeAudio[] = [];

class FakeAudio {
  src: string;
  play = vi.fn().mockResolvedValue(undefined);
  pause = vi.fn();
  constructor(src: string) {
    this.src = src;
    audios.push(this);
  }
}

const OriginalAudio = window.Audio;

function renderPanel() {
  const store = configureStore({ reducer: { mascot: mascotReducer } });
  return {
    store,
    ...render(
      <Provider store={store}>
        <MemoryRouter>
          <MascotPanel />
        </MemoryRouter>
      </Provider>
    ),
  };
}

const previewButton = () => screen.getByTestId('mascot-voice-preview');

/** A deferred so a preview can be held open across another interaction. */
function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  vi.clearAllMocks();
  audios.length = 0;
  useMascotManifestMock.mockReturnValue({
    manifest: {
      schemaVersion: 1,
      generatedAt: '',
      mascots: [],
      source: { repository: '', branch: '', commit: '' },
    },
    entry: null,
    loading: false,
    error: null,
  });
  window.Audio = FakeAudio as unknown as typeof Audio;
});

afterEach(() => {
  window.Audio = OriginalAudio;
});

describe('MascotPanel — voice preview plays the synthesized clip', () => {
  it('synthesizes with the effective voice id and plays the result', async () => {
    mockSynthesizeSpeech.mockResolvedValue({ audio_mime: 'audio/wav', audio_base64: 'QUJD' });
    renderPanel();

    fireEvent.click(previewButton());

    await waitFor(() => expect(audios).toHaveLength(1));
    expect(mockSynthesizeSpeech).toHaveBeenCalledTimes(1);
    expect(audios[0].src).toBe('data:audio/wav;base64,QUJD');
    expect(audios[0].play).toHaveBeenCalled();
  });

  it('falls back to audio/mpeg when the backend omits a mime type', async () => {
    // An empty `audio_mime` would otherwise produce `data:;base64,…`, which no
    // browser will play — and the failure is silent.
    mockSynthesizeSpeech.mockResolvedValue({ audio_mime: '', audio_base64: 'QUJD' });
    renderPanel();

    fireEvent.click(previewButton());

    await waitFor(() => expect(audios).toHaveLength(1));
    expect(audios[0].src).toBe('data:audio/mpeg;base64,QUJD');
  });

  it('disables the button while a preview is in flight and re-enables it after', async () => {
    const gate = deferred<{ audio_mime: string; audio_base64: string }>();
    mockSynthesizeSpeech.mockReturnValue(gate.promise);
    renderPanel();

    fireEvent.click(previewButton());
    await waitFor(() => expect(previewButton()).toBeDisabled());

    await act(async () => {
      gate.resolve({ audio_mime: 'audio/mpeg', audio_base64: 'QUJD' });
    });
    await waitFor(() => expect(previewButton()).toBeEnabled());
  });
});

describe('MascotPanel — a superseded preview does not talk over the new one', () => {
  // `onVoicePreview` guards its post-await work with
  // `if (previewRequestIdRef.current !== requestId) return;`. Only ONE of the
  // two things that bump that ref is reachable from the UI: the button
  // self-disables for the duration of a preview, so a second click cannot
  // overtake the first. Unmount is the reachable path, and it is the one that
  // matters — leaving Settings mid-preview must not start audio for a panel
  // that is gone, and must not touch state on it.
  it('creates no audio for a response that lands after the panel unmounted', async () => {
    const gate = deferred<{ audio_mime: string; audio_base64: string }>();
    mockSynthesizeSpeech.mockReturnValue(gate.promise);
    const { unmount } = renderPanel();

    fireEvent.click(previewButton());
    await waitFor(() => expect(mockSynthesizeSpeech).toHaveBeenCalled());

    // Counted rather than asserted-empty: what matters is that the late resolve
    // adds nothing, not what earlier tests in this file left behind.
    const before = audios.length;
    unmount();
    await act(async () => {
      gate.resolve({ audio_mime: 'audio/mpeg', audio_base64: 'TEFURQ==' });
    });

    expect(audios).toHaveLength(before);
  });

  it('plays nothing at all when the only preview is abandoned by unmounting', async () => {
    const gate = deferred<{ audio_mime: string; audio_base64: string }>();
    mockSynthesizeSpeech.mockReturnValue(gate.promise);
    const { unmount } = renderPanel();

    fireEvent.click(previewButton());
    await waitFor(() => expect(mockSynthesizeSpeech).toHaveBeenCalled());
    unmount();
    await act(async () => {
      gate.resolve({ audio_mime: 'audio/mpeg', audio_base64: 'TEFURQ==' });
    });

    expect(audios.some(a => a.play.mock.calls.length > 0)).toBe(false);
  });
});

describe('MascotPanel — a failed preview is explained', () => {
  it('surfaces the thrown message rather than a bare failure', async () => {
    mockSynthesizeSpeech.mockRejectedValue(new Error('tts provider unreachable'));
    renderPanel();

    fireEvent.click(previewButton());

    const alert = await screen.findByTestId('mascot-voice-preview-error');
    expect(alert).toHaveTextContent('tts provider unreachable');
  });

  it('re-enables the button after a failure so the user can retry', async () => {
    mockSynthesizeSpeech.mockRejectedValue(new Error('nope'));
    renderPanel();

    fireEvent.click(previewButton());

    await screen.findByTestId('mascot-voice-preview-error');
    expect(previewButton()).toBeEnabled();
  });

  it('clears a previous error when a later preview succeeds', async () => {
    mockSynthesizeSpeech.mockRejectedValueOnce(new Error('nope'));
    renderPanel();

    fireEvent.click(previewButton());
    await screen.findByTestId('mascot-voice-preview-error');

    mockSynthesizeSpeech.mockResolvedValueOnce({ audio_mime: 'audio/mpeg', audio_base64: 'QUJD' });
    fireEvent.click(previewButton());

    await waitFor(() =>
      expect(screen.queryByTestId('mascot-voice-preview-error')).not.toBeInTheDocument()
    );
  });
});
