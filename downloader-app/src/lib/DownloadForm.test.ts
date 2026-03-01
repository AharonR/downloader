import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import DownloadForm from './DownloadForm.svelte';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);
const listenMock = vi.mocked(listen);

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

async function enterInput(value: string) {
  const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
  await fireEvent.input(textarea, { target: { value } });
  return textarea;
}

describe('DownloadForm', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listenMock.mockResolvedValue(() => {});
    // Provide a default mock for list_projects (used by ProjectSelector on mount)
    invokeMock.mockImplementation((command) => {
      if (command === 'list_projects') return Promise.resolve([]);
      return Promise.resolve(undefined);
    });
  });

  it('renders a textarea and a download button', () => {
    render(DownloadForm);
    expect(screen.getByRole('textbox')).toBeDefined();
    expect(screen.getByRole('button', { name: /download/i })).toBeDefined();
  });

  it('download button is disabled when textarea is empty', () => {
    render(DownloadForm);
    const button = screen.getByRole('button', { name: /download/i }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it('download button is enabled when textarea has content', async () => {
    render(DownloadForm);
    await enterInput('https://example.com/paper.pdf');
    const button = screen.getByRole('button', { name: /download/i }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);
  });

  it('download button is disabled when textarea contains only whitespace', async () => {
    render(DownloadForm);
    await enterInput('   \n   ');
    const button = screen.getByRole('button', { name: /download/i }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it('submits trimmed line-split inputs to the progress command', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'list_projects') return Promise.resolve([]);
      return Promise.resolve({ completed: 2, failed: 0, output_dir: '/tmp/downloads' });
    });

    render(DownloadForm);
    await enterInput('  https://example.com/a.pdf  \n\n 10.1000/xyz123 \n   ');

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('start_download_with_progress', {
        inputs: ['https://example.com/a.pdf', '10.1000/xyz123'],
        project: null,
      });
    });
  });

  it('passes project name to start_download_with_progress when project field is filled', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'list_projects') return Promise.resolve([]);
      return Promise.resolve({ completed: 1, failed: 0, output_dir: '/tmp/downloads/Climate-Research' });
    });

    render(DownloadForm);

    const projectInput = screen.getByRole('combobox') as HTMLInputElement;
    await fireEvent.input(projectInput, { target: { value: 'Climate Research' } });

    await enterInput('https://example.com/paper.pdf');

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('start_download_with_progress', {
        inputs: ['https://example.com/paper.pdf'],
        project: 'Climate Research',
      });
    });
  });

  it('does not clear projectName when reset after a successful download', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'list_projects') return Promise.resolve([]);
      return Promise.resolve({ completed: 1, failed: 0, output_dir: '/tmp/downloads/Climate-Research' });
    });

    render(DownloadForm);

    // Fill in project name
    const projectInput = screen.getByRole('combobox') as HTMLInputElement;
    await fireEvent.input(projectInput, { target: { value: 'Climate Research' } });

    await enterInput('https://example.com/paper.pdf');
    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    // Wait for CompletionSummary's reset button to appear
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /download more/i })).toBeDefined();
    });

    // Click "Download more" (handleReset)
    await fireEvent.click(screen.getByRole('button', { name: /download more/i }));

    // projectName must survive the reset — intentional UX: user continues in the same project
    await waitFor(() => {
      const inputAfterReset = screen.getByRole('combobox') as HTMLInputElement;
      expect(inputAfterReset.value).toBe('Climate Research');
    });
  });

  it('registers the progress listener before invoking the download command', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'list_projects') return Promise.resolve([]);
      return Promise.resolve({ completed: 1, failed: 0, output_dir: '/tmp/downloads' });
    });

    render(DownloadForm);
    await enterInput('https://example.com/a.pdf');

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    await waitFor(() => {
      expect(listenMock).toHaveBeenCalledWith('download://progress', expect.any(Function));
      expect(invokeMock).toHaveBeenCalledWith('start_download_with_progress', expect.any(Object));
    });

    // listen must be called before start_download_with_progress
    const downloadCallIdx = invokeMock.mock.calls.findIndex(
      (call) => call[0] === 'start_download_with_progress',
    );
    expect(listenMock.mock.invocationCallOrder[0]).toBeLessThan(
      invokeMock.mock.invocationCallOrder[downloadCallIdx],
    );
  });

  it('shows downloading state immediately while invoke is pending', async () => {
    const pending = deferred<{ completed: number; failed: number; output_dir: string }>();
    invokeMock.mockReturnValue(pending.promise);

    render(DownloadForm);
    const textarea = await enterInput('https://example.com/a.pdf');

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    const button = screen.getByRole('button', { name: /downloading/i }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(textarea.disabled).toBe(true);
    expect(screen.getByText(/Resolving…/)).toBeTruthy();
    expect(screen.getByRole('button', { name: /cancel/i })).toBeDefined();

    pending.resolve({ completed: 1, failed: 0, output_dir: '/tmp/downloads' });
    await waitFor(() => {
      expect(screen.getByText(/Downloaded 1 file to/)).toBeTruthy();
    });
  });

  it('renders backend errors and exits downloading state on rejection', async () => {
    invokeMock.mockRejectedValue(
      'What: Download failed.\nWhy: Network unavailable.\nFix: Retry once network access is restored.',
    );

    render(DownloadForm);
    await enterInput('https://example.com/a.pdf');

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    await waitFor(() => {
      expect(screen.getByText(/What: Download failed\./)).toBeTruthy();
    });

    expect(screen.queryByRole('button', { name: /cancel/i })).toBeNull();
    expect(screen.getByRole('button', { name: /^Download$/i })).toBeDefined();
  });

  it('calls unlisten after a successful completion', async () => {
    const unlisten = vi.fn();
    listenMock.mockResolvedValue(unlisten);
    invokeMock.mockImplementation((command) => {
      if (command === 'list_projects') return Promise.resolve([]);
      return Promise.resolve({ completed: 1, failed: 0, output_dir: '/tmp/downloads' });
    });

    render(DownloadForm);
    await enterInput('https://example.com/a.pdf');

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    await waitFor(() => {
      expect(unlisten).toHaveBeenCalledOnce();
    });
  });

  it('calls unlisten after a rejected download attempt', async () => {
    const unlisten = vi.fn();
    listenMock.mockResolvedValue(unlisten);
    invokeMock.mockRejectedValue('What: Failed.\nWhy: Nope.\nFix: Retry.');

    render(DownloadForm);
    await enterInput('https://example.com/a.pdf');

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    await waitFor(() => {
      expect(unlisten).toHaveBeenCalledOnce();
    });
  });

  it('calls unlisten on component destroy while a download is pending', async () => {
    const unlisten = vi.fn();
    const pending = deferred<{ completed: number; failed: number; output_dir: string }>();
    listenMock.mockResolvedValue(unlisten);
    invokeMock.mockReturnValue(pending.promise);

    const { unmount } = render(DownloadForm);
    await enterInput('https://example.com/a.pdf');

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    await waitFor(() => {
      expect(listenMock).toHaveBeenCalled();
    });

    unmount();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('shows cancel only while downloading and invokes cancel once', async () => {
    const pending = deferred<{ completed: number; failed: number; output_dir: string }>();
    invokeMock.mockImplementation((command) => {
      if (command === 'cancel_download') {
        return Promise.resolve();
      }
      return pending.promise;
    });

    render(DownloadForm);
    await enterInput('https://example.com/a.pdf');

    expect(screen.queryByRole('button', { name: /cancel/i })).toBeNull();

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    const cancelButton = await screen.findByRole('button', { name: /cancel/i });
    expect(cancelButton).toBeDefined();

    await fireEvent.click(cancelButton);
    expect(invokeMock).toHaveBeenCalledWith('cancel_download');
    expect((cancelButton as HTMLButtonElement).disabled).toBe(true);

    await fireEvent.click(cancelButton);
    expect(invokeMock.mock.calls.filter(([command]) => command === 'cancel_download')).toHaveLength(
      1,
    );

    pending.resolve({ completed: 1, failed: 0, output_dir: '/tmp/downloads' });
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /cancel/i })).toBeNull();
    });
  });

  it('stays stable if cancel_download rejects', async () => {
    const pending = deferred<{ completed: number; failed: number; output_dir: string }>();
    invokeMock.mockImplementation((command) => {
      if (command === 'cancel_download') {
        return Promise.reject(new Error('cancel failed'));
      }
      return pending.promise;
    });

    render(DownloadForm);
    await enterInput('https://example.com/a.pdf');

    await fireEvent.submit(screen.getByRole('button', { name: /download/i }).closest('form')!);

    const cancelButton = await screen.findByRole('button', { name: /cancel/i });
    await fireEvent.click(cancelButton);

    expect((cancelButton as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByRole('button', { name: /downloading/i })).toBeDefined();

    pending.resolve({ completed: 1, failed: 0, output_dir: '/tmp/downloads' });
    await waitFor(() => {
      expect(screen.getByText(/Cancelled/)).toBeTruthy();
      expect(screen.getByText(/1 completed, 0 failed/)).toBeTruthy();
    });
  });
});
