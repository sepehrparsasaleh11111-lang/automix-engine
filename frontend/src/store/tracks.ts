import { create } from 'zustand';
import type { TrackSummary } from '../types';
import { importTracks, listTracks } from '../api/ipc';

interface TracksState {
  tracks: TrackSummary[];
  loading: boolean;
  load: (projectId: string | null) => Promise<void>;
  addFromPaths: (paths: string[], projectId: string | null) => Promise<void>;
}

export const useTracksStore = create<TracksState>((set, get) => ({
  tracks: [],
  loading: false,
  load: async (projectId) => {
    set({ loading: true });
    const tracks = await listTracks(projectId);
    set({ tracks, loading: false });
  },
  addFromPaths: async (paths, projectId) => {
    const imported = await importTracks(paths, projectId);
    set({ tracks: [...get().tracks, ...imported] });
  },
}));