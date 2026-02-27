import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import ProgressDisplay from './ProgressDisplay.svelte';

// ProgressDisplay does not call Tauri APIs directly — no mocks needed.

describe('ProgressDisplay', () => {
  const makePayload = (overrides = {}) => ({
    completed: 2,
    failed: 1,
    total: 5,
    in_progress: [],
    ...overrides,
  });

  it('shows completed/total counts', () => {
    render(ProgressDisplay, { props: { payload: makePayload() } });
    expect(screen.getByText(/2 \/ 5/)).toBeTruthy();
  });

  it('shows failed badge when failed > 0', () => {
    render(ProgressDisplay, { props: { payload: makePayload({ failed: 1 }) } });
    expect(screen.getByText(/1 failed/)).toBeTruthy();
  });

  it('does not show failed badge when failed === 0', () => {
    render(ProgressDisplay, { props: { payload: makePayload({ failed: 0 }) } });
    expect(screen.queryByText(/failed/)).toBeNull();
  });

  it('shows spinner while in-flight (completed + failed < total)', () => {
    render(ProgressDisplay, { props: { payload: makePayload({ completed: 2, failed: 0, total: 5 }) } });
    const spinner = document.querySelector('.spinner');
    expect(spinner).not.toBeNull();
  });

  it('hides spinner when all done', () => {
    render(ProgressDisplay, { props: { payload: makePayload({ completed: 5, failed: 0, total: 5 }) } });
    const spinner = document.querySelector('.spinner');
    expect(spinner).toBeNull();
  });

  it('renders in-progress items', () => {
    const payload = makePayload({
      in_progress: [
        { url: 'https://arxiv.org/abs/2301.00001', bytes_downloaded: 1_200_000, content_length: 3_400_000 },
      ],
    });
    render(ProgressDisplay, { props: { payload } });
    expect(screen.getByText(/arxiv\.org/)).toBeTruthy();
    expect(screen.getByText(/1\.1 MB \/ 3\.2 MB/)).toBeTruthy();
  });

  it('renders bytes only when content_length is null', () => {
    const payload = makePayload({
      in_progress: [
        { url: 'https://example.com/file.pdf', bytes_downloaded: 2048, content_length: null },
      ],
    });
    render(ProgressDisplay, { props: { payload } });
    expect(screen.getByText(/^2\.0 KB$/)).toBeTruthy();
    expect(screen.queryByText(/2\.0 KB \/ /)).toBeNull();
  });

  it('renders bytes only when content_length is zero', () => {
    const payload = makePayload({
      in_progress: [
        { url: 'https://example.com/file.pdf', bytes_downloaded: 2048, content_length: 0 },
      ],
    });
    render(ProgressDisplay, { props: { payload } });
    expect(screen.getByText(/^2\.0 KB$/)).toBeTruthy();
  });

  it('sets progress value to completed plus failed', () => {
    render(ProgressDisplay, {
      props: { payload: makePayload({ completed: 2, failed: 1, total: 5 }) },
    });
    const progress = screen.getByRole('progressbar') as HTMLProgressElement;
    expect(progress.value).toBe(3);
  });

  it('sets progress max to total', () => {
    render(ProgressDisplay, {
      props: { payload: makePayload({ completed: 2, failed: 1, total: 5 }) },
    });
    const progress = screen.getByRole('progressbar') as HTMLProgressElement;
    expect(progress.max).toBe(5);
  });

  it('omits the active downloads list when no items are in progress', () => {
    render(ProgressDisplay, { props: { payload: makePayload({ in_progress: [] }) } });
    expect(screen.queryByLabelText(/Active downloads/i)).toBeNull();
  });

  it('falls back to the raw string when an in-progress item URL is invalid', () => {
    const payload = makePayload({
      in_progress: [
        { url: 'not-a-url', bytes_downloaded: 1024, content_length: null },
      ],
    });
    render(ProgressDisplay, { props: { payload } });
    expect(screen.getByText(/^not-a-url$/)).toBeTruthy();
  });
});
