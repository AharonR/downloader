import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import CompletionSummary from './CompletionSummary.svelte';

describe('CompletionSummary', () => {
  const makeSummary = (overrides = {}) => ({
    completed: 3,
    failed: 0,
    output_dir: '/home/user/downloads',
    ...overrides,
  });

  it('success path: shows downloaded count and output dir', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary(), onReset: vi.fn() },
    });
    expect(screen.getByText(/Downloaded 3 files to/)).toBeTruthy();
    expect(screen.getByText(/\/home\/user\/downloads/)).toBeTruthy();
  });

  it('success path: singular "file" when completed === 1', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ completed: 1 }), onReset: vi.fn() },
    });
    expect(screen.getByText(/Downloaded 1 file to/)).toBeTruthy();
  });

  it('success path: keeps the reset button available', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary(), onReset: vi.fn() },
    });
    expect(screen.getByRole('button', { name: /download more/i })).toBeTruthy();
  });

  it('partial path: shows completed + failed counts', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ completed: 2, failed: 1 }), onReset: vi.fn() },
    });
    expect(screen.getByText(/2 downloaded, 1 failed/)).toBeTruthy();
  });

  it('partial path: shows error-hint when failed > 0', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ completed: 2, failed: 1 }), onReset: vi.fn() },
    });
    expect(screen.getByText(/Some downloads failed/)).toBeTruthy();
  });

  it('partial path: omits output dir when nothing completed', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ completed: 0, failed: 2 }), onReset: vi.fn() },
    });
    expect(screen.queryByText(/saved to/)).toBeNull();
    expect(screen.queryByText(/\/home\/user\/downloads/)).toBeNull();
  });

  it('cancel path: shows cancelled status', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ completed: 1, failed: 0 }), onReset: vi.fn(), cancelled: true },
    });
    expect(screen.getByText(/Cancelled/)).toBeTruthy();
    expect(screen.getByText(/1 completed, 0 failed/)).toBeTruthy();
  });

  it('cancel path: shows error-hint when failed > 0 after cancel', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ completed: 1, failed: 2 }), onReset: vi.fn(), cancelled: true },
    });
    expect(screen.getByText(/did not complete before cancellation/)).toBeTruthy();
  });

  it('cancel path: no error-hint when failed === 0 after cancel', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ completed: 3, failed: 0 }), onReset: vi.fn(), cancelled: true },
    });
    expect(screen.queryByText(/did not complete before cancellation/)).toBeNull();
  });

  it('calls onReset when "Download more" is clicked', async () => {
    const onReset = vi.fn();
    render(CompletionSummary, { props: { summary: makeSummary(), onReset } });
    await fireEvent.click(screen.getByRole('button', { name: /download more/i }));
    expect(onReset).toHaveBeenCalledOnce();
  });
});
