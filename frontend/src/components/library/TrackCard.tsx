import { WaveformCanvas } from '../waveform/WaveformCanvas';
import type { TrackSummary } from '../../types';

function formatDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, '0')}`;
}

export function TrackCard({ track }: { track: TrackSummary }) {
  return (
    <div className="rounded border p-3" data-testid="track-card">
      <div className="flex items-center justify-between">
        <div>
          <span className="font-medium">{track.title}</span>
          <span className="ml-2 text-gray-500">{track.artist ?? 'Unknown artist'}</span>
        </div>
        <div className="flex items-center gap-2">
          <span className="rounded bg-gray-200 px-1.5 py-0.5 text-xs text-gray-700">
            {track.format}
          </span>
          <span className="text-gray-500">{formatDuration(track.duration_ms)}</span>
        </div>
      </div>
      <WaveformCanvas peaks={track.peaks} />
    </div>
  );
}