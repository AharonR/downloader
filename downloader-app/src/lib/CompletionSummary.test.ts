import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import CompletionSummary from './CompletionSummary.svelte';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe('CompletionSummary', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

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
    expect(screen.getByText(/Run complete/)).toBeTruthy();
    expect(screen.getByText(/\/home\/user\/downloads/)).toBeTruthy();
  });

  it('success path: shows ready state when completed === 1', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ completed: 1 }), onReset: vi.fn() },
    });
    expect(screen.getByText(/Ready/)).toBeTruthy();
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
    expect(screen.getByText(/Run complete with items to review/)).toBeTruthy();
    expect(screen.getByText(/Needs attention/)).toBeTruthy();
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
    expect(screen.getByText(/Run stopped/)).toBeTruthy();
    expect(screen.getByText(/Cancelled/)).toBeTruthy();
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

  it('renders structured post-run warnings when present', () => {
    render(CompletionSummary, {
      props: {
        summary: makeSummary({
          warnings: [
            {
              code: 'registry_persist_failed',
              path: '/tmp/project/.downloader/downloaded-registry.v1.json',
              error: 'Permission denied',
              impact: 'future runs may re-download',
              fix: 'Check write permissions and rerun.',
            },
          ],
        }),
        onReset: vi.fn(),
      },
    });

    expect(screen.getByText(/Warnings/)).toBeTruthy();
    expect(screen.getByText(/registry_persist_failed/)).toBeTruthy();
    expect(screen.getByText(/Permission denied/)).toBeTruthy();
    expect(screen.getByText(/future runs may re-download/)).toBeTruthy();
  });

  it('shows "Open output folder" button when output_dir is non-empty and files completed', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/home/user/papers', completed: 2 }), onReset: vi.fn() },
    });
    expect(screen.getByRole('button', { name: /open output folder/i })).toBeTruthy();
  });

  it('hides "Open output folder" button when no files completed', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/home/user/papers', completed: 0, failed: 2 }), onReset: vi.fn() },
    });
    expect(screen.queryByRole('button', { name: /open output folder/i })).toBeNull();
  });

  it('invokes open_folder with the correct path when "Open output folder" is clicked', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const invokeMock = vi.mocked(invoke);

    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/home/user/papers' }), onReset: vi.fn() },
    });

    await fireEvent.click(screen.getByRole('button', { name: /open output folder/i }));

    expect(invokeMock).toHaveBeenCalledWith('open_folder', { path: '/home/user/papers' });
  });

  it('shows an error message when open_folder rejects', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const invokeMock = vi.mocked(invoke);
    invokeMock.mockRejectedValueOnce('Permission denied');

    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/home/user/papers' }), onReset: vi.fn() },
    });

    await fireEvent.click(screen.getByRole('button', { name: /open output folder/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeTruthy();
      expect(screen.getByText(/Permission denied/)).toBeTruthy();
    });
  });

  // -------------------------------------------------------------------------
  // Convert HTML → PDF button
  // -------------------------------------------------------------------------

  it('shows "Convert HTML → PDF" button when output_dir is set and files completed', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/tmp/out', completed: 3 }), onReset: vi.fn() },
    });
    expect(screen.getByRole('button', { name: /convert html → pdf/i })).toBeTruthy();
  });

  it('does not show convert button when no files completed or skipped', () => {
    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/tmp/out', completed: 0, failed: 2 }), onReset: vi.fn() },
    });
    expect(screen.queryByRole('button', { name: /convert html/i })).toBeNull();
  });

  it('convert button transitions to "Converting…" and is disabled after click', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockReturnValueOnce(new Promise(() => {})); // never resolves

    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/tmp/out', completed: 3 }), onReset: vi.fn() },
    });

    await fireEvent.click(screen.getByRole('button', { name: /convert html → pdf/i }));

    await waitFor(() => {
      const btn = screen.getByRole('button', { name: /converting/i });
      expect(btn).toBeTruthy();
      expect((btn as HTMLButtonElement).disabled).toBe(true);
    });
  });

  it('shows converted count when invoke resolves with results', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValueOnce({ converted: 3, skipped: 1, failed: 0, total: 4 });

    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/tmp/out', completed: 4 }), onReset: vi.fn() },
    });

    await fireEvent.click(screen.getByRole('button', { name: /convert html → pdf/i }));

    await waitFor(() => {
      expect(screen.getByText(/Converted 3 files/)).toBeTruthy();
      expect(screen.getByText(/1 skipped/)).toBeTruthy();
    });
  });

  it('shows "Nothing to convert" when converted and failed are both zero', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValueOnce({ converted: 0, skipped: 0, failed: 0, total: 0 });

    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/tmp/out', completed: 3 }), onReset: vi.fn() },
    });

    await fireEvent.click(screen.getByRole('button', { name: /convert html → pdf/i }));

    await waitFor(() => {
      expect(screen.getByText(/Nothing to convert/)).toBeTruthy();
    });
  });

  it('shows failed count alongside converted when some files failed', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValueOnce({ converted: 2, skipped: 0, failed: 1, total: 3 });

    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/tmp/out', completed: 3 }), onReset: vi.fn() },
    });

    await fireEvent.click(screen.getByRole('button', { name: /convert html → pdf/i }));

    await waitFor(() => {
      expect(screen.getByText(/Converted 2 files/)).toBeTruthy();
      expect(screen.getByText(/1 failed/)).toBeTruthy();
    });
  });

  it('shows inline error when Chrome is not found', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockRejectedValueOnce('Chrome not found. Install Google Chrome or set DOWNLOADER_CHROME_BINARY.');

    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/tmp/out', completed: 3 }), onReset: vi.fn() },
    });

    await fireEvent.click(screen.getByRole('button', { name: /convert html → pdf/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeTruthy();
      expect(screen.getByText(/Chrome not found/)).toBeTruthy();
    });
  });

  it('resets convert state when "Download more" is clicked after conversion', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValueOnce({ converted: 2, skipped: 0, failed: 0, total: 2 });

    const onReset = vi.fn();
    render(CompletionSummary, {
      props: { summary: makeSummary({ output_dir: '/tmp/out', completed: 2 }), onReset },
    });

    await fireEvent.click(screen.getByRole('button', { name: /convert html → pdf/i }));
    await waitFor(() => expect(screen.getByText(/Converted 2 files/)).toBeTruthy());

    await fireEvent.click(screen.getByRole('button', { name: /download more/i }));
    expect(onReset).toHaveBeenCalledOnce();

    // convertState should be back to 'idle' — the "Convert HTML → PDF" button is the evidence.
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /convert html → pdf/i })).toBeTruthy();
    });
  });
});
