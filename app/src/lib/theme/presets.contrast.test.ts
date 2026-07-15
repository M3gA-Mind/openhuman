import { describe, expect, it } from 'vitest';

import { channelContrast } from './color';
import { PRESET_THEMES } from './presets';
import type { Theme } from './types';

/**
 * WCAG AA gate for every shipped **dark** preset.
 *
 * A preset only carries the tokens it overrides; everything else falls through
 * to the `:root.dark` defaults in `app/src/styles/tokens.css`. Those defaults are
 * not importable as JS, so we mirror the relevant subset here and resolve each
 * theme by merging its `colors` over this base — the same layering ThemeProvider
 * does at runtime. Keep this in sync with tokens.css `:root.dark`.
 */
const DARK_BASE: Record<string, string> = {
  surface: '23 23 23',
  'surface-canvas': '0 0 0',
  'surface-muted': '38 38 38',
  'surface-subtle': '38 38 38',
  'surface-strong': '38 38 38',
  'surface-hover': '38 38 38',
  'surface-overlay': '0 0 0',
  content: '245 245 245',
  'content-secondary': '212 212 212',
  'content-muted': '163 163 163',
  'content-faint': '115 115 115',
  'content-inverted': '255 255 255',
  'primary-200': '191 219 254',
  'primary-300': '147 197 253',
  'primary-400': '96 165 250',
  'primary-500': '47 110 244',
  'primary-600': '37 99 235',
  'primary-700': '29 78 216',
};

const AA_TEXT = 4.5; // body text
const AA_LARGE = 3.0; // large text / UI elements / disabled-placeholder

/** All surface layers text can land on, including hover/pressed states. */
const SURFACES = [
  'surface',
  'surface-canvas',
  'surface-muted',
  'surface-subtle',
  'surface-strong',
  'surface-hover',
] as const;

/** Text tiers held to full body contrast against every surface. */
const BODY_TIERS = ['content', 'content-secondary', 'content-muted'] as const;

function resolve(theme: Theme): Record<string, string> {
  return { ...DARK_BASE, ...theme.colors };
}

const darkPresets = PRESET_THEMES.filter(t => t.isDark && t.builtIn);

describe('preset dark themes meet WCAG AA', () => {
  it('ships the expected dark presets', () => {
    // Guards against a preset being renamed/dropped without updating this gate.
    expect(darkPresets.map(t => t.id).sort()).toEqual(
      ['dark', 'hal9000', 'matrix', 'ocean-dark', 'sepia-dark'].sort()
    );
  });

  for (const theme of darkPresets) {
    describe(`${theme.name} (${theme.id})`, () => {
      const t = resolve(theme);

      it('body/muted text ≥ 4.5:1 on every surface state', () => {
        for (const surface of SURFACES) {
          for (const tier of BODY_TIERS) {
            const ratio = channelContrast(t[tier], t[surface]);
            expect(
              ratio,
              `${theme.id}: ${tier} on ${surface} = ${ratio.toFixed(2)}`
            ).toBeGreaterThanOrEqual(AA_TEXT);
          }
        }
      });

      it('faint/placeholder text ≥ 3:1 on every surface state', () => {
        for (const surface of SURFACES) {
          const ratio = channelContrast(t['content-faint'], t[surface]);
          expect(
            ratio,
            `${theme.id}: content-faint on ${surface} = ${ratio.toFixed(2)}`
          ).toBeGreaterThanOrEqual(AA_LARGE);
        }
      });

      it('primary button label ≥ 4.5:1 on its resting and active fills', () => {
        // Button.tsx: bg-primary-500 (rest) / dark:active:bg-primary-600, label
        // is text-content-inverted.
        for (const shade of ['primary-500', 'primary-600'] as const) {
          const ratio = channelContrast(t['content-inverted'], t[shade]);
          expect(
            ratio,
            `${theme.id}: content-inverted on ${shade} = ${ratio.toFixed(2)}`
          ).toBeGreaterThanOrEqual(AA_TEXT);
        }
      });

      it('accent/link text ≥ 4.5:1 on surface and canvas', () => {
        // Dark-mode accent text uses the lighter shades (dark:text-primary-200…400).
        for (const shade of ['primary-200', 'primary-300', 'primary-400'] as const) {
          for (const surface of ['surface', 'surface-canvas'] as const) {
            const ratio = channelContrast(t[shade], t[surface]);
            expect(
              ratio,
              `${theme.id}: ${shade} text on ${surface} = ${ratio.toFixed(2)}`
            ).toBeGreaterThanOrEqual(AA_TEXT);
          }
        }
      });

      it('primary-500 reads as a UI element ≥ 3:1 on surface', () => {
        // Focus ring (focus-visible:ring-primary-500) and control boundaries.
        const ratio = channelContrast(t['primary-500'], t.surface);
        expect(ratio, `${theme.id}: primary-500 vs surface = ${ratio.toFixed(2)}`).toBeGreaterThanOrEqual(
          AA_LARGE
        );
      });
    });
  }
});
