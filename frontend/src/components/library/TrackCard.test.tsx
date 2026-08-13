import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TrackCard } from './TrackCard';
import type { TrackSummary } from '../../types';

const track: TrackSummary = {
  id: 't1',
  path: '/a.mp3',
  title: 'Neon Nights',
  artist: 'DJ Test',
  album: null,
  duration_ms: 210_000,
  sample_rate: 44100,
  channels: 2,
  format: 'mp3',
  peaks: [{ min: -0.5, max: 0.5 }],
};

describe('TrackCard', () => {
  it('renders title, artist and duration', () => {
    render(<TrackCard track={track} />);
    expect(screen.getByText('Neon Nights')).toBeInTheDocument();
    expect(screen.getByText('DJ Test')).toBeInTheDocument();
    expect(screen.getByText('3:30')).toBeInTheDocument();
    expect(screen.getByText('mp3')).toBeInTheDocument();
  });
});