import { describe, expect, test } from 'vitest';

import {
  buildFallbackHeadline,
  buildShareCaption,
  redactSensitive,
  sanitizeHeadline,
  SHARE_HEADLINE_MAX,
  stripMarkdown,
  truncateAtWord,
  TWEET_MAX,
} from './shareContent';

describe('redactSensitive', () => {
  test('scrubs API keys', () => {
    expect(redactSensitive('key sk-abcdef0123456789ABCDEF here')).toContain('[redacted]');
    expect(redactSensitive('key sk-abcdef0123456789ABCDEF here')).not.toContain('sk-abcdef');
  });

  test('scrubs POSIX and Windows paths', () => {
    expect(redactSensitive('saved to /Users/jane/secret.txt')).toContain('[path]');
    expect(redactSensitive('at C:\\Users\\jane\\notes.md')).toContain('[path]');
    expect(redactSensitive('saved to /Users/jane/secret.txt')).not.toContain('jane');
  });

  test('scrubs email addresses', () => {
    expect(redactSensitive('mail jane.doe@example.com now')).toContain('[email]');
  });

  test('is idempotent and leaves clean prose untouched', () => {
    const clean = 'Summarised three months of emails in twelve seconds';
    expect(redactSensitive(clean)).toBe(clean);
    expect(redactSensitive(redactSensitive(clean))).toBe(clean);
  });
});

describe('stripMarkdown', () => {
  test('removes fences, emphasis, headings, and links', () => {
    const md = '# Title\n\n**Bold** and _italic_ with [a link](http://x.io)\n```\ncode\n```';
    const out = stripMarkdown(md);
    expect(out).not.toContain('#');
    expect(out).not.toContain('**');
    expect(out).not.toContain('```');
    expect(out).toContain('a link');
    expect(out).not.toContain('http://x.io');
  });
});

describe('truncateAtWord', () => {
  test('leaves short text unchanged', () => {
    expect(truncateAtWord('hello world', 50)).toBe('hello world');
  });

  test('never exceeds max and ends with an ellipsis when cut', () => {
    const out = truncateAtWord('the quick brown fox jumps over the lazy dog', 20);
    expect(out.length).toBeLessThanOrEqual(20);
    expect(out.endsWith('…')).toBe(true);
  });
});

describe('buildFallbackHeadline', () => {
  test('takes the first sentence and caps length', () => {
    const out = buildFallbackHeadline('Summarised the report. Then did more.');
    expect(out).toBe('Summarised the report');
    expect(out.length).toBeLessThanOrEqual(SHARE_HEADLINE_MAX);
  });

  test('returns empty for empty input', () => {
    expect(buildFallbackHeadline('   ')).toBe('');
  });

  test('redacts secrets that appear in output', () => {
    expect(buildFallbackHeadline('Wrote key to /Users/jane/.env file')).not.toContain('jane');
  });
});

describe('sanitizeHeadline', () => {
  test('strips wrapping quotes and trailing punctuation', () => {
    expect(sanitizeHeadline('"My agent shipped a feature!"')).toBe('My agent shipped a feature');
  });

  test('returns empty for too-short garbage', () => {
    expect(sanitizeHeadline('.')).toBe('');
  });

  test('caps at the headline max', () => {
    const long = 'word '.repeat(80);
    expect(sanitizeHeadline(long).length).toBeLessThanOrEqual(SHARE_HEADLINE_MAX);
  });
});

describe('buildShareCaption', () => {
  test('embeds the headline and brand', () => {
    const out = buildShareCaption('My agent summarised my inbox');
    expect(out).toContain('My agent summarised my inbox');
    expect(out).toContain('OpenHuman');
  });

  test('falls back to a generic caption for empty headline', () => {
    expect(buildShareCaption('')).toContain('OpenHuman');
  });

  test('leaves headroom under the tweet limit', () => {
    const out = buildShareCaption('x'.repeat(400));
    expect(out.length).toBeLessThanOrEqual(TWEET_MAX - 30);
  });
});
