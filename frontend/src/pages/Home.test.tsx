import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import Home from './Home';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectsStore } from '../store/projects';

describe('Home', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(open).mockReset();
    useProjectsStore.setState({ projects: [], selectedId: null, loading: false });
  });

  it('renders projects from the store', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_projects')
        return Promise.resolve([{ id: 'p1', name: 'Mix 1', created_at: '', updated_at: '' }]);
      if (cmd === 'list_tracks') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(<Home />);
    expect(await screen.findByText('Mix 1')).toBeInTheDocument();
  });

  it('creates a project on submit', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_projects') return Promise.resolve([]);
      if (cmd === 'create_project')
        return Promise.resolve({ id: 'p2', name: 'New Mix', created_at: '', updated_at: '' });
      if (cmd === 'list_tracks') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(<Home />);
    const input = await screen.findByPlaceholderText('Project name');
    fireEvent.change(input, { target: { value: 'New Mix' } });
    fireEvent.click(screen.getByRole('button', { name: /create/i }));
    expect(await screen.findByText('New Mix')).toBeInTheDocument();
  });

  it('imports tracks via file dialog', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_projects')
        return Promise.resolve([{ id: 'p1', name: 'Mix 1', created_at: '', updated_at: '' }]);
      if (cmd === 'list_tracks') return Promise.resolve([]);
      if (cmd === 'import_tracks')
        return Promise.resolve([
          {
            id: 't1',
            path: '/a.wav',
            title: 'Imported',
            artist: null,
            album: null,
            duration_ms: 1000,
            sample_rate: 44100,
            channels: 1,
            format: 'wav',
            peaks: [{ min: -0.5, max: 0.5 }],
          },
        ]);
      return Promise.resolve([]);
    });
    vi.mocked(open).mockResolvedValue(['/a.wav'] as never);
    render(<Home />);
    fireEvent.click(await screen.findByRole('button', { name: /import/i }));
    expect(await screen.findByText('Imported')).toBeInTheDocument();
  });
});