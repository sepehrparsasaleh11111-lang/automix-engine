import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectsStore } from '../store/projects';
import { useTracksStore } from '../store/tracks';

function formatDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, '0')}`;
}

export default function Home() {
  const {
    projects,
    selectedId,
    load,
    create,
    remove,
    select,
  } = useProjectsStore();
  const { tracks, load: loadTracks, addFromPaths } = useTracksStore();
  const [name, setName] = useState('');

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    if (selectedId) loadTracks(selectedId);
  }, [selectedId, loadTracks]);

  const onImport = async () => {
    const paths = await open({
      multiple: true,
      filters: [{ name: 'Audio', extensions: ['mp3', 'wav', 'flac'] }],
    });
    if (paths) addFromPaths(paths, selectedId);
  };

  return (
    <div className="flex h-screen flex-col">
      <header className="border-b px-4 py-3 text-lg font-semibold">
        <h1>OpenMix AI</h1>
      </header>
      <div className="flex flex-1 overflow-hidden">
        <aside className="w-64 border-r p-4">
          <h2 className="mb-2 text-sm font-medium uppercase tracking-wide text-gray-500">Projects</h2>
          <div className="mb-3 flex gap-2">
            <input
              data-testid="project-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Project name"
              className="w-full rounded border px-2 py-1 text-sm"
            />
            <button
              onClick={() => {
                if (!name.trim()) return;
                create(name.trim());
                setName('');
              }}
              className="rounded bg-blue-600 px-2 py-1 text-sm text-white"
            >
              Create
            </button>
          </div>
          <ul>
            {projects.map((p) => (
              <li key={p.id} className="mb-1 flex items-center justify-between">
                <button
                  onClick={() => select(p.id)}
                  className={`flex-1 rounded px-2 py-1 text-left text-sm ${
                    selectedId === p.id ? 'bg-blue-100 font-medium' : ''
                  }`}
                >
                  {p.name}
                </button>
                <button
                  onClick={() => remove(p.id)}
                  aria-label={`Delete ${p.name}`}
                  className="px-2 text-gray-400 hover:text-red-500"
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        </aside>
        <main className="flex-1 overflow-y-auto p-4">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-xl font-semibold">Tracks</h2>
            <button
              onClick={onImport}
              className="rounded bg-green-600 px-3 py-1.5 text-sm text-white"
            >
              Import tracks
            </button>
          </div>
          <ul>
            {tracks.map((t) => (
              <li key={t.id} className="mb-1 rounded border px-3 py-2">
                <span className="font-medium">{t.title}</span>
                {t.artist && <span className="ml-2 text-gray-500">{t.artist}</span>}
                <span className="ml-auto text-right text-gray-500">
                  {formatDuration(t.duration_ms)}
                </span>
              </li>
            ))}
          </ul>
        </main>
      </div>
    </div>
  );
}