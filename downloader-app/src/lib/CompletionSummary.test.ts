import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import CompletionSummary from './CompletionSummary.svelte';

describe('CompletionSummary', () => {
  const makeSummary = (overrides = {}) => ({
    completed: 3,
    failed: 0,
    output_dir: '/home/user/downloads',
    failed_items: [],
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

  it('partial path: shows toggle button when failed_items is non-empty', () => {
    render(CompletionSummary, {
      props: {
        summary: makeSummary({
          completed: 2,
          failed: 1,
          failed_items: [{ input: 'https://bad.example.com/paper.pdf', error: 'HTTP 403' }],
        }),
        onReset: vi.fn(),
      },
    });
    expect(screen.getByRole('button', { name: /show failed items \(1\)/i })).toBeTruthy();
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

  it('cancel path: shows toggle button when failed_items is non-empty after cancel', () => {
    render(CompletionSummary, {
      props: {
        summary: makeSummary({
          completed: 1,
          failed: 2,
          failed_items: [
            { input: 'https://a.example.com/1.pdf', error: 'Interrupted' },
            { input: 'https://b.example.com/2.pdf', error: 'Interrupted' },
          ],
        }),
        onReset: vi.fn(),
        cancelled: true,
      },
    });
    expect(screen.getByRole('button', { name: /show failed items \(2\)/i })).toBeTruthy();
  });

  it('cancel path: no toggle button when failed_items is empty after cancel', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ completed: 3, failed: 0, failed_items: [] }), onReset: vi.fn(), cancelled: true },
    });
    expect(screen.queryByRole('button', { name: /failed items/i })).toBeNull();
  });

  it('clicking toggle expands to show per-item error details', async () => {
    render(CompletionSummary, {
      props: {
        summary: makeSummary({
          completed: 1,
          failed: 1,
          failed_items: [{ input: 'https://fail.example.com/paper.pdf', error: 'HTTP 404 Not Found' }],
        }),
        onReset: vi.fn(),
      },
    });

    // Details hidden initially
    expect(screen.queryByText(/HTTP 404 Not Found/)).toBeNull();

    // Click toggle
    await fireEvent.click(screen.getByRole('button', { name: /show failed items/i }));

    // Details now visible
    expect(screen.getByText(/HTTP 404 Not Found/)).toBeTruthy();
    expect(screen.getByText(/https:\/\/fail\.example\.com\/paper\.pdf/)).toBeTruthy();

    // Button label flips
    expect(screen.getByRole('button', { name: /hide failed items/i })).toBeTruthy();
  });

  it('calls onReset when "Download more" is clicked', async () => {
    const onReset = vi.fn();
    render(CompletionSummary, { props: { summary: makeSummary(), onReset } });
    await fireEvent.click(screen.getByRole('button', { name: /download more/i }));
    expect(onReset).toHaveBeenCalledOnce();
  });
});
