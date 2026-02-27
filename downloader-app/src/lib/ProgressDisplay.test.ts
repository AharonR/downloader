import { describe, it, expect, vi } from 'vitest';
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
});
