import { create } from 'zustand';
import type { Project } from '../types';
import { createProject as createProjectIpc, deleteProject, listProjects } from '../api/ipc';

interface ProjectsState {
  projects: Project[];
  selectedId: string | null;
  loading: boolean;
  load: () => Promise<void>;
  create: (name: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  select: (id: string) => void;
}

export const useProjectsStore = create<ProjectsState>((set, get) => ({
  projects: [],
  selectedId: null,
  loading: false,
  load: async () => {
    set({ loading: true });
    const projects = await listProjects();
    const selectedId = get().selectedId ?? (projects[0]?.id ?? null);
    set({ projects, selectedId, loading: false });
  },
  create: async (name) => {
    const project = await createProjectIpc(name);
    set({ projects: [...get().projects, project], selectedId: project.id });
  },
  remove: async (id) => {
    await deleteProject(id);
    const rest = get().projects.filter((p) => p.id !== id);
    const selectedId = get().selectedId === id ? (rest[0]?.id ?? null) : get().selectedId;
    set({ projects: rest, selectedId });
  },
  select: (id) => set({ selectedId: id }),
}));