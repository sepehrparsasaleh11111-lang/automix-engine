import { invoke } from '@tauri-apps/api/core';
import type { Project, TrackSummary } from '../types';

export const listProjects = () => invoke<Project[]>('list_projects');
export const createProject = (name: string) => invoke<Project>('create_project', { name });
export const deleteProject = (id: string) => invoke<void>('delete_project', { id });
export const listTracks = (projectId: string | null) =>
  invoke<TrackSummary[]>('list_tracks', { projectId });
export const importTracks = (paths: string[], projectId: string | null) =>
  invoke<TrackSummary[]>('import_tracks', { paths, projectId });