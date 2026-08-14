import { test } from 'node:test';
import assert from 'node:assert';

function calculateColumnWidth(canvasWidth: number, cols: number, gutterWidth: number): number {
  if (cols <= 0) return canvasWidth;
  const availableWidth = canvasWidth - (cols - 1) * gutterWidth;
  return Math.floor(availableWidth / cols);
}

function clampDimensions(width: number, height: number): { width: number; height: number } {
  const minW = 260;
  const minH = 180;
  return {
    width: Math.max(minW, width),
    height: Math.max(minH, height)
  };
}

test('TilingMath - Column width distribution across densities', () => {
  const canvasWidth = 1440;
  const gutter = 14;

  const w1 = calculateColumnWidth(canvasWidth, 1, gutter);
  assert.strictEqual(w1, 1440);

  const w2 = calculateColumnWidth(canvasWidth, 2, gutter);
  assert.strictEqual(w2, (1440 - 14) / 2);

  const w3 = calculateColumnWidth(canvasWidth, 3, gutter);
  assert.strictEqual(w3, Math.floor((1440 - 28) / 3));

  const w4 = calculateColumnWidth(canvasWidth, 4, gutter);
  assert.strictEqual(w4, Math.floor((1440 - 42) / 4));
});

test('TilingMath - Dimension boundary clamping', () => {
  const clamped1 = clampDimensions(100, 50);
  assert.strictEqual(clamped1.width, 260);
  assert.strictEqual(clamped1.height, 180);

  const clamped2 = clampDimensions(500, 400);
  assert.strictEqual(clamped2.width, 500);
  assert.strictEqual(clamped2.height, 400);
});
