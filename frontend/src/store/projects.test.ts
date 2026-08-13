import { describe, expect, it, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { useProjectsStore } from './projects';

describe('useProjectsStore', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    useProjectsStore.setState({ projects: [], selectedId: null, loading: false });
  });

  it('loads projects from IPC', async () => {
    vi.mocked(invoke).mockResolvedValue([{ id: 'p1', name: 'Mix 1', created_at: '', updated_at: '' }]);
    await useProjectsStore.getState().load();
    expect(invoke).toHaveBeenCalledWith('list_projects');
    expect(useProjectsStore.getState().projects).toHaveLength(1);
  });

  it('creates a project via IPC', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'p2', name: 'New', created_at: '', updated_at: '' });
    await useProjectsStore.getState().create('New');
    expect(invoke).toHaveBeenCalledWith('create_project', { name: 'New' });
    expect(useProjectsStore.getState().projects[0].name).toBe('New');
  });
});