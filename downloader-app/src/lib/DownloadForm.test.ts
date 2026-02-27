import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import DownloadForm from './DownloadForm.svelte';

// Mock Tauri IPC — not available in jsdom
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock Tauri event API — listen() is used by DownloadForm for progress events
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn(),
}));

describe('DownloadForm', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
    await fireEvent.input(textarea, { target: { value: 'https://example.com/paper.pdf' } });
    const button = screen.getByRole('button', { name: /download/i }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);
  });

  it('download button is disabled when textarea contains only whitespace', async () => {
    render(DownloadForm);
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
    await fireEvent.input(textarea, { target: { value: '   \n   ' } });
    const button = screen.getByRole('button', { name: /download/i }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });
});
