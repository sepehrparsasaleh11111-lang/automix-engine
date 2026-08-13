import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import App from './App';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';

describe('App', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_projects') return Promise.resolve([]);
      if (cmd === 'list_tracks') return Promise.resolve([]);
      return Promise.resolve([]);
    });
  });

  it('renders the app title', () => {
    render(<App />);
    expect(screen.getByRole('heading', { name: 'OpenMix AI' })).toBeInTheDocument();
  });
});