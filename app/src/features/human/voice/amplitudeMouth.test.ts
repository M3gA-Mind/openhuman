import { describe, expect, it } from 'vitest';

import { amplitudeToVisemeCode, normalizeAmplitude, smoothAmplitude } from './amplitudeMouth';

describe('normalizeAmplitude', () => {
  it('treats silence / sub-floor RMS as a closed mouth', () => {
    expect(normalizeAmplitude(0)).toBe(0);
    expect(normalizeAmplitude(0.015)).toBe(0);
    expect(normalizeAmplitude(0.01)).toBe(0);
  });

  it('clamps loud RMS to a fully-open mouth', () => {
    expect(normalizeAmplitude(0.18)).toBe(1);
    expect(normalizeAmplitude(0.5)).toBe(1);
  });

  it('maps the mid-range linearly between floor and ceiling', () => {
    // Midpoint of [0.015, 0.18] ≈ 0.0975 → ~0.5.
    expect(normalizeAmplitude(0.0975)).toBeCloseTo(0.5, 2);
  });

  it('is defensive against non-finite input', () => {
    expect(normalizeAmplitude(Number.NaN)).toBe(0);
    expect(normalizeAmplitude(Number.POSITIVE_INFINITY)).toBe(1);
  });
});

describe('smoothAmplitude', () => {
  it('eases toward the target rather than snapping', () => {
    const next = smoothAmplitude(0, 1);
    expect(next).toBeGreaterThan(0);
    expect(next).toBeLessThan(1);
  });

  it('opens faster than it closes (attack > release)', () => {
    const opening = smoothAmplitude(0, 1); // attack
    const closing = 1 - smoothAmplitude(1, 0); // release magnitude
    expect(opening).toBeGreaterThan(closing);
  });

  it('snaps tiny residuals to a fully-closed mouth', () => {
    expect(smoothAmplitude(0.02, 0)).toBe(0);
  });
});

describe('amplitudeToVisemeCode', () => {
  it('buckets the mouth-open value from closed to wide open', () => {
    expect(amplitudeToVisemeCode(0)).toBe('sil');
    expect(amplitudeToVisemeCode(0.1)).toBe('sil');
    expect(amplitudeToVisemeCode(0.25)).toBe('ih');
    expect(amplitudeToVisemeCode(0.5)).toBe('E');
    expect(amplitudeToVisemeCode(0.9)).toBe('aa');
  });
});
