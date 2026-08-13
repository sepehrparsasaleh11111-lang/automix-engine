import { describe, expect, it, vi } from 'vitest';
import { drawPeaks } from './drawPeaks';
import type { Peak } from '../../types';

function stubCtx() {
  const calls: string[] = [];
  return {
    calls,
    beginPath: vi.fn(() => calls.push('beginPath')),
    moveTo: vi.fn(() => calls.push('moveTo')),
    lineTo: vi.fn(() => calls.push('lineTo')),
    stroke: vi.fn(() => calls.push('stroke')),
  } as unknown as CanvasRenderingContext2D;
}

describe('drawPeaks', () => {
  it('draws one vertical line per peak', () => {
    const ctx = stubCtx();
    const peaks: Peak[] = [
      { min: -0.5, max: 0.5 },
      { min: -1, max: 1 },
    ];
    drawPeaks(ctx, peaks, 100, 100, '#fff');
    const lineToCalls = (ctx as unknown as { calls: string[] }).calls.filter((c) => c === 'lineTo');
    expect(lineToCalls).toHaveLength(2);
  });

  it('is a no-op for empty peaks', () => {
    const ctx = stubCtx();
    drawPeaks(ctx, [], 100, 100, '#fff');
    expect((ctx as unknown as { calls: string[] }).calls).toEqual([]);
  });
});