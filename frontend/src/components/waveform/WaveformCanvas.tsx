import { useEffect, useRef } from 'react';
import { drawPeaks } from './drawPeaks';
import type { Peak } from '../../types';

interface WaveformCanvasProps {
  peaks: Peak[];
  className?: string;
}

export function WaveformCanvas({ peaks, className }: WaveformCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, height);
    drawPeaks(ctx, peaks, width, height, '#34d399');
  }, [peaks]);

  return (
    <canvas
      ref={canvasRef}
      data-testid="waveform"
      className={className}
      style={{ width: '100%', height: '48px', display: 'block' }}
    />
  );
}