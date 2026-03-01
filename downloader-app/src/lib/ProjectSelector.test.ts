import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import ProjectSelector from './ProjectSelector.svelte';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

describe('ProjectSelector', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders a text input with the correct placeholder', () => {
    invokeMock.mockResolvedValue([]);
    render(ProjectSelector);
    const input = screen.getByRole('combobox') as HTMLInputElement;
    expect(input).toBeDefined();
    expect(input.placeholder).toBe('e.g. Climate Research');
  });

  it('renders a datalist linked to the input', () => {
    invokeMock.mockResolvedValue([]);
    render(ProjectSelector);
    const input = screen.getByRole('combobox') as HTMLInputElement;
    expect(input.list).toBeDefined();
    expect(input.getAttribute('list')).toBe('project-suggestions');
  });

  it('populates datalist options from list_projects invoke', async () => {
    invokeMock.mockResolvedValue(['Climate Research', 'Genomics Study']);
    render(ProjectSelector);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('list_projects');
    });

    const datalist = document.getElementById('project-suggestions') as HTMLDataListElement;
    expect(datalist).toBeDefined();
    const options = datalist.querySelectorAll('option');
    expect(options).toHaveLength(2);
    expect(options[0].value).toBe('Climate Research');
    expect(options[1].value).toBe('Genomics Study');
  });

  it('renders no datalist options when list_projects returns empty array', async () => {
    invokeMock.mockResolvedValue([]);
    render(ProjectSelector);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('list_projects');
    });

    const datalist = document.getElementById('project-suggestions') as HTMLDataListElement;
    expect(datalist.querySelectorAll('option')).toHaveLength(0);
  });

  it('renders no datalist options when list_projects rejects', async () => {
    invokeMock.mockRejectedValue(new Error('not in tauri'));
    render(ProjectSelector);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('list_projects');
    });

    const datalist = document.getElementById('project-suggestions') as HTMLDataListElement;
    expect(datalist.querySelectorAll('option')).toHaveLength(0);
  });

  it('disables the input when disabled prop is true', () => {
    invokeMock.mockResolvedValue([]);
    render(ProjectSelector, { props: { disabled: true } });
    const input = screen.getByRole('combobox') as HTMLInputElement;
    expect(input.disabled).toBe(true);
  });
});
