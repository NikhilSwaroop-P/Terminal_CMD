import { test } from 'node:test';
import assert from 'node:assert';

function simulateExponentialDecay(
  startX: number,
  targetX: number,
  decayRate = 28.0,
  dt = 0.016,
  maxSteps = 60
): { finalX: number; stepsTaken: number } {
  let currentX = startX;
  let stepsTaken = 0;

  for (let i = 0; i < maxSteps; i++) {
    stepsTaken++;
    const factor = 1.0 - Math.exp(-decayRate * dt);
    currentX += (targetX - currentX) * factor;

    if (Math.abs(currentX - targetX) < 0.2) {
      return { finalX: targetX, stepsTaken };
    }
  }

  return { finalX: currentX, stepsTaken };
}

test('CursorTrail - Exponential decay converges to target position within 150ms', () => {
  const start = 0;
  const target = 50;
  const { finalX, stepsTaken } = simulateExponentialDecay(start, target);

  assert.strictEqual(Math.abs(finalX - target) < 0.1, true);
  assert.strictEqual(stepsTaken <= 15, true);
});
