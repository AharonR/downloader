import { describe, it, expect } from 'vitest';
import { formatBytes, urlDomain } from './utils.js';

describe('formatBytes', () => {
  it('formats bytes < 1024 as B', () => {
    expect(formatBytes(512)).toBe('512 B');
  });
  it('returns "0 B" for negative values', () => {
    expect(formatBytes(-1)).toBe('0 B');
  });
  it('returns "0 B" for NaN', () => {
    expect(formatBytes(NaN)).toBe('0 B');
  });
  it('formats KB', () => {
    expect(formatBytes(2048)).toBe('2.0 KB');
  });
  it('formats MB', () => {
    expect(formatBytes(1_200_000)).toBe('1.1 MB');
  });
  it('formats GB', () => {
    expect(formatBytes(1_073_741_824)).toBe('1.00 GB');
  });
});

describe('urlDomain', () => {
  it('extracts hostname from URL', () => {
    expect(urlDomain('https://arxiv.org/abs/2301.00001')).toBe('arxiv.org');
  });
  it('returns raw string on invalid URL', () => {
    expect(urlDomain('not-a-url')).toBe('not-a-url');
  });
});
