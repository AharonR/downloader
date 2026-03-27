import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import AuthPanel from './AuthPanel.svelte';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe('AuthPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
      }
      return Promise.resolve(undefined);
    });
  });

  // -----------------------------------------------------------------------
  // Rendering & toggle
  // -----------------------------------------------------------------------

  it('renders the toggle button with label', () => {
    render(AuthPanel);
    expect(screen.getByRole('button', { name: /publisher authentication/i })).toBeDefined();
  });

  it('loads cookie status on mount', async () => {
    render(AuthPanel);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('get_cookie_status');
    });
  });

  it('does not show the body when collapsed', () => {
    render(AuthPanel);
    expect(screen.queryByText(/export cookies/i)).toBeNull();
    expect(screen.queryByPlaceholderText(/paste/i)).toBeNull();
  });

  it('shows the body when toggle is clicked', async () => {
    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));
    expect(screen.getByPlaceholderText(/paste/i)).toBeDefined();
    expect(screen.getByRole('button', { name: /import from paste/i })).toBeDefined();
    expect(screen.getByRole('button', { name: /import cookies\.txt file/i })).toBeDefined();
  });

  it('collapses back when toggle is clicked again', async () => {
    render(AuthPanel);
    const toggle = screen.getByRole('button', { name: /publisher authentication/i });
    await fireEvent.click(toggle);
    expect(screen.getByPlaceholderText(/paste/i)).toBeDefined();

    await fireEvent.click(toggle);
    expect(screen.queryByPlaceholderText(/paste/i)).toBeNull();
  });

  // -----------------------------------------------------------------------
  // Badge when cookies exist
  // -----------------------------------------------------------------------

  it('shows domain count badge when cookies are saved', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({
          has_cookies: true,
          domain_count: 2,
          domains: ['emerald.com', 'wiley.com'],
        });
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await waitFor(() => {
      expect(screen.getByText('2 domains')).toBeDefined();
    });
  });

  it('shows singular "domain" for single domain', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({
          has_cookies: true,
          domain_count: 1,
          domains: ['emerald.com'],
        });
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await waitFor(() => {
      expect(screen.getByText('1 domain')).toBeDefined();
    });
  });

  // -----------------------------------------------------------------------
  // Cookie status display (expanded)
  // -----------------------------------------------------------------------

  it('shows saved domain names when expanded and cookies exist', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({
          has_cookies: true,
          domain_count: 2,
          domains: ['emerald.com', 'wiley.com'],
        });
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await waitFor(() => {
      expect(screen.getByText('2 domains')).toBeDefined();
    });

    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));
    expect(screen.getByText(/emerald\.com, wiley\.com/)).toBeDefined();
    expect(screen.getByRole('button', { name: /clear cookies/i })).toBeDefined();
  });

  it('does not show status bar when no cookies are saved', async () => {
    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));
    expect(screen.queryByRole('button', { name: /clear cookies/i })).toBeNull();
  });

  // -----------------------------------------------------------------------
  // Paste import
  // -----------------------------------------------------------------------

  it('import-from-paste button is disabled when textarea is empty', async () => {
    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));

    const importBtn = screen.getByRole('button', { name: /import from paste/i }) as HTMLButtonElement;
    expect(importBtn.disabled).toBe(true);
  });

  it('import-from-paste button is enabled when textarea has content', async () => {
    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));

    const textarea = screen.getByPlaceholderText(/paste/i);
    await fireEvent.input(textarea, { target: { value: '.emerald.com\tTRUE\t/\tFALSE\t0\tsid\tabc' } });

    const importBtn = screen.getByRole('button', { name: /import from paste/i }) as HTMLButtonElement;
    expect(importBtn.disabled).toBe(false);
  });

  it('calls import_cookies with pasted text and shows success feedback', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
      }
      if (command === 'import_cookies') {
        return Promise.resolve({
          domain_count: 1,
          cookie_count: 3,
          warnings: [],
          storage_path: '/home/user/.config/downloader/cookies.enc',
        });
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));

    const textarea = screen.getByPlaceholderText(/paste/i);
    await fireEvent.input(textarea, { target: { value: 'cookie data here' } });
    await fireEvent.click(screen.getByRole('button', { name: /import from paste/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('import_cookies', { input: 'cookie data here' });
      expect(screen.getByText(/Saved 3 cookies for 1 domain/)).toBeDefined();
    });
  });

  it('clears textarea after successful paste import', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
      }
      if (command === 'import_cookies') {
        return Promise.resolve({
          domain_count: 1,
          cookie_count: 2,
          warnings: [],
          storage_path: '/tmp/cookies.enc',
        });
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));

    const textarea = screen.getByPlaceholderText(/paste/i) as HTMLTextAreaElement;
    await fireEvent.input(textarea, { target: { value: 'cookie data' } });
    await fireEvent.click(screen.getByRole('button', { name: /import from paste/i }));

    await waitFor(() => {
      expect(textarea.value).toBe('');
    });
  });

  it('shows error feedback when import_cookies rejects', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
      }
      if (command === 'import_cookies') {
        return Promise.reject('What: Could not parse cookie data.\nWhy: cookie input is empty\nFix: Export cookies...');
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));

    const textarea = screen.getByPlaceholderText(/paste/i);
    await fireEvent.input(textarea, { target: { value: 'bad data' } });
    await fireEvent.click(screen.getByRole('button', { name: /import from paste/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeDefined();
      expect(screen.getByText(/Could not parse cookie data/)).toBeDefined();
    });
  });

  it('refreshes cookie status after successful import', async () => {
    let statusCallCount = 0;
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        statusCallCount += 1;
        if (statusCallCount <= 1) {
          return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
        }
        return Promise.resolve({
          has_cookies: true,
          domain_count: 1,
          domains: ['emerald.com'],
        });
      }
      if (command === 'import_cookies') {
        return Promise.resolve({
          domain_count: 1,
          cookie_count: 5,
          warnings: [],
          storage_path: '/tmp/cookies.enc',
        });
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('get_cookie_status');
    });

    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));

    const textarea = screen.getByPlaceholderText(/paste/i);
    await fireEvent.input(textarea, { target: { value: 'cookie data' } });
    await fireEvent.click(screen.getByRole('button', { name: /import from paste/i }));

    await waitFor(() => {
      expect(screen.getByText('1 domain')).toBeDefined();
    });
  });

  // -----------------------------------------------------------------------
  // File import
  // -----------------------------------------------------------------------

  it('calls import_cookies_from_file and shows success feedback', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
      }
      if (command === 'import_cookies_from_file') {
        return Promise.resolve({
          domain_count: 2,
          cookie_count: 8,
          warnings: [],
          storage_path: '/tmp/cookies.enc',
        });
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));
    await fireEvent.click(screen.getByRole('button', { name: /import cookies\.txt file/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('import_cookies_from_file');
      expect(screen.getByText(/Saved 8 cookies for 2 domains/)).toBeDefined();
    });
  });

  it('suppresses feedback when file picker is cancelled', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
      }
      if (command === 'import_cookies_from_file') {
        return Promise.reject('What: No file selected.\nWhy: The file picker was cancelled.\nFix: Try again...');
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));
    await fireEvent.click(screen.getByRole('button', { name: /import cookies\.txt file/i }));

    // Should NOT show an error alert for a cancelled picker
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('import_cookies_from_file');
    });
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('shows error feedback when file import fails for non-cancel reasons', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
      }
      if (command === 'import_cookies_from_file') {
        return Promise.reject('What: Could not read cookie file.\nWhy: Permission denied.\nFix: Check file permissions.');
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));
    await fireEvent.click(screen.getByRole('button', { name: /import cookies\.txt file/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeDefined();
      expect(screen.getByText(/Could not read cookie file/)).toBeDefined();
    });
  });

  // -----------------------------------------------------------------------
  // Clear cookies
  // -----------------------------------------------------------------------

  it('calls clear_cookies and shows success feedback', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({
          has_cookies: true,
          domain_count: 1,
          domains: ['emerald.com'],
        });
      }
      if (command === 'clear_cookies') {
        return Promise.resolve(true);
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await waitFor(() => {
      expect(screen.getByText('1 domain')).toBeDefined();
    });

    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));
    await fireEvent.click(screen.getByRole('button', { name: /clear cookies/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('clear_cookies');
      expect(screen.getByText(/Cookies cleared/)).toBeDefined();
    });
  });

  // -----------------------------------------------------------------------
  // Disabled states during import
  // -----------------------------------------------------------------------

  it('disables buttons while importing', async () => {
    const pending = deferred<{ domain_count: number; cookie_count: number; warnings: string[]; storage_path: string }>();
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
      }
      if (command === 'import_cookies') {
        return pending.promise;
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));

    const textarea = screen.getByPlaceholderText(/paste/i);
    await fireEvent.input(textarea, { target: { value: 'cookie data' } });

    await fireEvent.click(screen.getByRole('button', { name: /import from paste/i }));

    // While importing, buttons should be disabled
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /importing/i })).toBeDefined();
    });
    const fileBtn = screen.getByRole('button', { name: /import cookies\.txt file/i }) as HTMLButtonElement;
    expect(fileBtn.disabled).toBe(true);

    pending.resolve({ domain_count: 1, cookie_count: 1, warnings: [], storage_path: '/tmp/c.enc' });
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /import from paste/i })).toBeDefined();
    });
  });

  // -----------------------------------------------------------------------
  // Graceful handling of get_cookie_status failure
  // -----------------------------------------------------------------------

  it('renders without badge when get_cookie_status rejects', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.reject(new Error('not in tauri'));
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('get_cookie_status');
    });

    // No badge, no crash
    expect(screen.queryByText(/domain/)).toBeNull();
    expect(screen.getByRole('button', { name: /publisher authentication/i })).toBeDefined();
  });

  // -----------------------------------------------------------------------
  // Plural formatting
  // -----------------------------------------------------------------------

  it('uses correct singular/plural for cookie count', async () => {
    invokeMock.mockImplementation((command) => {
      if (command === 'get_cookie_status') {
        return Promise.resolve({ has_cookies: false, domain_count: 0, domains: [] });
      }
      if (command === 'import_cookies') {
        return Promise.resolve({
          domain_count: 1,
          cookie_count: 1,
          warnings: [],
          storage_path: '/tmp/cookies.enc',
        });
      }
      return Promise.resolve(undefined);
    });

    render(AuthPanel);
    await fireEvent.click(screen.getByRole('button', { name: /publisher authentication/i }));

    const textarea = screen.getByPlaceholderText(/paste/i);
    await fireEvent.input(textarea, { target: { value: 'data' } });
    await fireEvent.click(screen.getByRole('button', { name: /import from paste/i }));

    await waitFor(() => {
      expect(screen.getByText(/Saved 1 cookie for 1 domain\./)).toBeDefined();
    });
  });
});
