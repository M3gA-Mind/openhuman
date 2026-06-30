/**
 * Amplitude-driven mouth mapping for Tiny's lip-sync (Option A, #4077).
 *
 * The TTS audio is decoded into an RMS energy envelope ahead of playback (see
 * {@link ./audioPlayer.ts}); these pure helpers turn a raw RMS sample into a
 * mouth-open value and a discrete viseme bucket, so the mascot's mouth opens
 * and closes in time with the actual loudness of the speech.
 *
 * This is intentionally phoneme-free: it does not try to match vowel/consonant
 * shapes (that is the follow-up viseme-driven work). It only answers "how open
 * should the mouth be right now, given how loud the audio is right now."
 *
 * All functions are pure so they can be unit-tested without a DOM / Web Audio.
 */

/** Raw RMS below this is treated as silence → mouth closed. */
const SILENCE_FLOOR = 0.015;
/** Raw RMS at/above this maps to a fully-open mouth. */
const OPEN_CEILING = 0.18;

/**
 * Normalise a raw RMS amplitude (0..1, typically ~0.0–0.3 for speech PCM) into
 * a 0..1 mouth-open value, with a noise floor (so quiet hiss/room tone doesn't
 * flap the mouth) and a ceiling (so normal speech reaches a fully-open mouth).
 */
export function normalizeAmplitude(rms: number): number {
  // Non-finite energy (NaN / ±Infinity from corrupt input) is meaningless →
  // closed mouth. Real decoded-PCM RMS is always finite in [0, 1], so this is
  // purely defensive; treat every non-finite value uniformly rather than
  // letting +Infinity fall through to the fully-open ceiling clamp (a jammed-
  // wide-open mouth is a worse failure mode than a closed one).
  if (!Number.isFinite(rms) || rms <= SILENCE_FLOOR) return 0;
  if (rms >= OPEN_CEILING) return 1;
  return (rms - SILENCE_FLOOR) / (OPEN_CEILING - SILENCE_FLOOR);
}

/**
 * One-pole smoothing of the mouth-open value across animation frames. The
 * mouth opens quickly (fast attack) so onsets feel snappy, and closes a little
 * slower (slower release) so it doesn't chatter between every zero-crossing.
 * `prev` and `target` are both normalised 0..1 values.
 */
export function smoothAmplitude(
  prev: number,
  target: number,
  attack = 0.6,
  release = 0.25
): number {
  const k = target > prev ? attack : release;
  const next = prev + (target - prev) * k;
  // Snap tiny residuals to 0 so the mouth fully closes on silence. The release
  // step from a near-floor value (e.g. 0.02 → 0) leaves a ~0.015 residual; this
  // threshold collapses that to a fully-closed mouth instead of a lingering gap.
  return next < 0.02 ? 0 : next;
}

/**
 * Map a normalised mouth-open value (0..1) to an Oculus/Rive viseme code so the
 * discrete Rive state machine (which speaks in viseme enums, not a continuous
 * openness) gets a closed → wide-open bucket. Codes are members of the Rive
 * 15-set (`sil`, `ih`, `E`, `aa`) and are also understood by the SVG mouth.
 */
export function amplitudeToVisemeCode(open: number): string {
  if (open < 0.12) return 'sil';
  if (open < 0.4) return 'ih';
  if (open < 0.7) return 'E';
  return 'aa';
}
