import { mockIPC } from '@tauri-apps/api/mocks';
import type { Project, TrackSummary } from '../types';

export function installMocks() {
  const projects: Project[] = [{ id: 'p1', name: 'Demo Mix', created_at: '', updated_at: '' }];
  const tracks: TrackSummary[] = [
    {
      id: 't1',
      path: '/demo.mp3',
      title: 'E2E Track',
      artist: 'E2E Artist',
      album: null,
      duration_ms: 90_000,
      sample_rate: 44100,
      channels: 2,
      format: 'mp3',
      peaks: Array.from({ length: 200 }, (_, i) => ({
        min: -Math.abs(Math.sin(i / 10)),
        max: Math.abs(Math.sin(i / 10)),
      })),
    },
  ];

  mockIPC((cmd, args) => {
    switch (cmd) {
      case 'list_projects':
        return Promise.resolve(projects);
      case 'create_project':
        return Promise.resolve({
          id: 'p2',
          name: (args as { name: string }).name,
          created_at: '',
          updated_at: '',
        });
      case 'delete_project':
        return Promise.resolve(undefined);
      case 'list_tracks':
        return Promise.resolve((args as { projectId: string | null }).projectId === null ? [] : tracks);
      case 'import_tracks':
        return Promise.resolve(tracks);
      default:
        return Promise.resolve(undefined);
    }
  });
}