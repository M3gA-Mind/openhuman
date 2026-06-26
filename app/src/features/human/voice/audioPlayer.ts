/**
 * Lightweight base64 → playable HTMLAudio wrapper. We don't need WebAudio
 * graph here; the viseme scheduler reads `currentTime` directly.
 */
import debug from 'debug';

const audioLog = debug('human:audio-player');

/**
 * Sentinel thrown by the `ended` promise when `stop()` is called. Callers that
 * intentionally cancel playback (e.g. swapping to a new utterance) can detect
 * this and silence the rejection without masking real audio decoder errors.
 */
export class AudioStoppedError extends Error {
  readonly stopped = true;
  constructor() {
    super('stopped');
    this.name = 'AudioStoppedError';
  }
}

export function isAudioStopped(err: unknown): err is AudioStoppedError {
  if (err instanceof AudioStoppedError) return true;
  return typeof err === 'object' && err !== null && (err as { stopped?: unknown }).stopped === true;
}

/**
 * Use as `.catch(swallowAudioStop)` on an orphan `handle.ended` promise, or
 * `catch (err) { swallowAudioStop(err); }` around an awaited one. Swallows the
 * stop sentinel; rethrows anything else so real decoder errors stay visible.
 */
export function swallowAudioStop(err: unknown): void {
  if (isAudioStopped(err)) return;
  throw err;
}

/**
 * Frames-per-second of the precomputed RMS amplitude envelope. ~60 Hz (≈16ms
 * per window) keeps the mouth within the <100ms audio-offset budget while
 * staying cheap to compute. Used both when building the envelope and when
 * sampling it by playback time.
 */
const ENVELOPE_HZ = 60;

export interface PlaybackHandle {
  /** ms elapsed since audio started. Returns -1 after playback ends. */
  currentMs(): number;
  /**
   * Raw RMS amplitude (0..1) of the audio at the current playback position,
   * read from a precomputed energy envelope (see {@link buildRmsEnvelope}).
   * Returns 0 before the envelope is ready, after playback ends, or when Web
   * Audio decoding was unavailable — callers normalise/smooth this into a
   * mouth-open value. Best-effort: never throws, never affects playback. (#4077)
   */
  amplitude(): number;
  /**
   * Total audio duration in ms. Returns 0 if `loadedmetadata` has not fired
   * yet — call again after a tick or wait on `metadataReady`. A function (not
   * a static field) so callers always read the latest value rather than a
   * stale snapshot taken before the decoder finished probing.
   */
  durationMs(): number;
  /** Resolves once the decoder reports duration (or the safety timeout fires). */
  metadataReady: Promise<void>;
  /** Stop playback and release the blob URL. Idempotent. */
  stop(): void;
  /** Resolves when the audio finishes naturally. Rejects if `stop()` is called. */
  ended: Promise<void>;
}

/**
 * Options for {@link playBase64Audio}.
 */
export interface PlaybackOptions {
  /**
   * Auto-stop playback after this many milliseconds as a safety bound against
   * runaway TTS. If the audio finishes naturally before the deadline the timer
   * is cleared — the option has no effect on normal, reasonably-sized replies.
   * When omitted, no duration limit is applied.
   */
  maxDurationMs?: number;
}

/**
 * Decode `bytes` into a mono RMS energy envelope sampled at {@link ENVELOPE_HZ},
 * for amplitude-driven lip-sync (#4077). Best-effort and side-effect-free with
 * respect to playback: it decodes a SEPARATE copy of the audio in a throwaway
 * `AudioContext` (closed immediately after) and never touches the playing
 * `HTMLAudioElement`, so there is zero risk of muting or perturbing TTS output
 * — if Web Audio is unavailable or decoding fails, it simply resolves to null
 * and the mouth falls back to the viseme scheduler.
 */
async function buildRmsEnvelope(bytes: Uint8Array): Promise<Float32Array | null> {
  try {
    const Ctor =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!Ctor) return null;
    const ctx = new Ctor();
    try {
      // `decodeAudioData` detaches the backing buffer, so hand it a fresh copy.
      const copy = bytes.slice().buffer as ArrayBuffer;
      const buffer = await ctx.decodeAudioData(copy);
      const channel = buffer.getChannelData(0);
      const windowSize = Math.max(1, Math.round(buffer.sampleRate / ENVELOPE_HZ));
      const frameCount = Math.max(1, Math.ceil(channel.length / windowSize));
      const envelope = new Float32Array(frameCount);
      for (let f = 0; f < frameCount; f++) {
        const start = f * windowSize;
        const end = Math.min(start + windowSize, channel.length);
        let sumSquares = 0;
        for (let i = start; i < end; i++) {
          const sample = channel[i];
          sumSquares += sample * sample;
        }
        envelope[f] = Math.sqrt(sumSquares / Math.max(1, end - start));
      }
      return envelope;
    } finally {
      void ctx.close?.();
    }
  } catch (err) {
    audioLog(
      'amplitude envelope unavailable: %s',
      err instanceof Error ? err.message : String(err)
    );
    return null;
  }
}

export async function playBase64Audio(
  base64: string,
  mime: string = 'audio/mpeg',
  options: PlaybackOptions = {}
): Promise<PlaybackHandle> {
  const { maxDurationMs } = options;
  const bytes = Uint8Array.from(atob(base64), c => c.charCodeAt(0));
  const blob = new Blob([bytes], { type: mime });
  const url = URL.createObjectURL(blob);
  const audio = new window.Audio(url);
  audio.preload = 'auto';

  // Kick off amplitude-envelope decoding in parallel — do NOT await before
  // `audio.play()` below, since any await between the originating user gesture
  // and play() can trip CEF/Chromium's autoplay policy. The envelope simply
  // becomes available a few ms into playback; `amplitude()` returns 0 until then.
  let amplitudeEnvelope: Float32Array | null = null;
  void buildRmsEnvelope(bytes).then(env => {
    amplitudeEnvelope = env;
  });

  let stopped = false;
  let endedNaturally = false;
  let resolveEnded!: () => void;
  let rejectEnded!: (err: Error) => void;
  const ended = new Promise<void>((res, rej) => {
    resolveEnded = res;
    rejectEnded = rej;
  });

  // Max-duration safety timer — cleared on any terminal event so it never
  // fires after playback has already ended.
  let durationTimer: number | null = null;

  const cleanup = () => {
    if (durationTimer != null) {
      window.clearTimeout(durationTimer);
      durationTimer = null;
    }
    URL.revokeObjectURL(url);
  };

  audio.addEventListener('ended', () => {
    endedNaturally = true;
    audioLog('playback ended naturally mime=%s', mime);
    cleanup();
    resolveEnded();
  });
  audio.addEventListener('error', () => {
    audioLog('playback decoder error mime=%s', mime);
    cleanup();
    rejectEnded(new Error('audio playback error'));
  });

  // Track metadata readiness without awaiting before `play()`: CEF/Chromium's
  // autoplay policy keys off the synchronous gesture chain, and any `await`
  // between the originating user click and `audio.play()` invalidates it,
  // causing play() to reject with "the user didn't interact with the document
  // first." We capture duration in a side listener and let the caller wait
  // on `metadataReady` separately if it needs it.
  let resolveMetadata!: () => void;
  const metadataReady = new Promise<void>(res => {
    resolveMetadata = res;
  });
  audio.addEventListener(
    'loadedmetadata',
    () => {
      audioLog('playback metadata ready duration=%ss mime=%s', audio.duration.toFixed(2), mime);
      resolveMetadata();
    },
    { once: true }
  );
  // Safety timeout so the procedural-viseme fallback never blocks forever if
  // the decoder skips `loadedmetadata` (some MP3 streams) — fall through to
  // the text-length estimate path in that case.
  window.setTimeout(() => resolveMetadata(), 500);

  const stop = () => {
    if (stopped) return;
    stopped = true;
    audio.pause();
    cleanup();
    rejectEnded(new AudioStoppedError());
  };

  try {
    await audio.play();
  } catch (err) {
    cleanup();
    rejectEnded(err instanceof Error ? err : new Error(String(err)));
    throw err;
  }

  audioLog('playback started mime=%s maxDurationMs=%s', mime, maxDurationMs ?? 'none');

  // Arm the max-duration safety timer after `play()` succeeds. Fires only if
  // the audio hasn't ended naturally by then (e.g. very long TTS response or a
  // decoder that never emits `ended`).
  if (maxDurationMs != null) {
    durationTimer = window.setTimeout(() => {
      durationTimer = null;
      if (!stopped && !endedNaturally) {
        audioLog(
          'playback auto-stopped after maxDurationMs=%d — preventing runaway TTS',
          maxDurationMs
        );
        stop();
      }
    }, maxDurationMs);
  }

  return {
    currentMs: () => (endedNaturally || stopped ? -1 : audio.currentTime * 1000),
    amplitude: () => {
      if (endedNaturally || stopped || amplitudeEnvelope === null) return 0;
      const idx = Math.floor((audio.currentTime * 1000) / (1000 / ENVELOPE_HZ));
      if (idx < 0 || idx >= amplitudeEnvelope.length) return 0;
      return amplitudeEnvelope[idx];
    },
    durationMs: () => (Number.isFinite(audio.duration) ? audio.duration * 1000 : 0),
    metadataReady,
    stop,
    ended,
  };
}
