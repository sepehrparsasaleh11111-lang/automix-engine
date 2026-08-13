import type { Peak } from '../../types';

export function drawPeaks(
  ctx: CanvasRenderingContext2D,
  peaks: Peak[],
  width: number,
  height: number,
  color: string,
): void {
  if (peaks.length === 0) return;
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  ctx.beginPath();
  const mid = height / 2;
  const step = width / peaks.length;
  peaks.forEach((p, i) => {
    const x = i * step;
    const yMin = mid + p.min * mid;
    const yMax = mid + p.max * mid;
    ctx.moveTo(x, yMin);
    ctx.lineTo(x, yMax);
  });
  ctx.stroke();
}