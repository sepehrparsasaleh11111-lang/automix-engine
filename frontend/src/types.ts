export interface Peak {
  min: number;
  max: number;
}

export interface Project {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface TrackSummary {
  id: string;
  path: string;
  title: string;
  artist: string | null;
  album: string | null;
  duration_ms: number;
  sample_rate: number;
  channels: number;
  format: string;
  peaks: Peak[];
}