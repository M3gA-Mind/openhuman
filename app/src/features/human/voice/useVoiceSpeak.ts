import { useEffect, useRef } from 'react';

import debug from 'debug';

import { socketService } from '../../../services/socketService';

import { type PlaybackHandle, playBase64Audio, swallowAudioStop } from './audioPlayer';
import { synthesizeSpeech } from './ttsClient';

const log = debug('human:voice-speak');

/** Hard cap on a single spoken prompt, guarding against runaway TTS. */
const MAX_SPEAK_MS = 20_000;

/** Payload of the core `voice:speak` socket event (mirrors `SpeakRequest`). */
interface SpeakPayload {
  text: string;
  source?: string | null;
}

function isSpeakPayload(value: unknown): value is SpeakPayload {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as { text?: unknown }).text === 'string'
  );
}

/**
 * Play proactive assistant speech requested by the core via `voice:speak`.
 *
 * Today's only producer is the voice-native approval surface: when a sensitive
 * action is parked for approval during a voice-initiated turn, the core asks the
 * assistant to speak the confirmation aloud so a hands-free user can answer
 * "yes"/"no" by voice (Phase 4 of #3148). Mounted once, app-wide, so the prompt
 * is heard even when the mascot view isn't open — it synthesizes through the
 * same TTS path the mascot uses and plays the returned audio directly.
 */
export function useVoiceSpeak(): void {
  const handleRef = useRef<PlaybackHandle | null>(null);

  useEffect(() => {
    const onSpeak = (...args: unknown[]): void => {
      const payload = args[0];
      if (!isSpeakPayload(payload)) return;
      const text = payload.text.trim();
      if (!text) return;
      log('voice:speak source=%s chars=%d', payload.source ?? 'unknown', text.length);

      void (async () => {
        try {
          const { audio_base64: audioBase64, audio_mime: audioMime } = await synthesizeSpeech(text);
          if (!audioBase64) return;
          // Stop any in-flight prompt before starting the next one.
          handleRef.current?.stop();
          const handle = await playBase64Audio(audioBase64, audioMime || 'audio/mpeg', {
            maxDurationMs: MAX_SPEAK_MS,
          });
          handleRef.current = handle;
          handle.ended.catch(swallowAudioStop);
        } catch (err) {
          log('voice:speak playback failed: %o', err);
        }
      })();
    };

    socketService.on('voice:speak', onSpeak);
    return () => {
      socketService.off('voice:speak', onSpeak);
      handleRef.current?.stop();
      handleRef.current = null;
    };
  }, []);
}
