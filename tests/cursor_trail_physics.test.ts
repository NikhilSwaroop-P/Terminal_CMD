import { test } from 'node:test';
import assert from 'node:assert';

function simulateSpringDamper(
  startX: number,
  targetX: number,
  stiffness = 240.0,
  damping = 18.0,
  mass = 1.0,
  dt = 0.016,
  maxSteps = 80
): { finalX: number; stepsTaken: number } {
  let currentX = startX;
  let velocityX = 0;
  let stepsTaken = 0;

  for (let i = 0; i < maxSteps; i++) {
    stepsTaken++;
    const displacement = currentX - targetX;
    const force = -stiffness * displacement - damping * velocityX;
    const accel = force / mass;

    velocityX += accel * dt;
    currentX += velocityX * dt;

    if (Math.abs(currentX - targetX) < 0.05 && Math.abs(velocityX) < 0.05) {
      return { finalX: targetX, stepsTaken };
    }
  }

  return { finalX: currentX, stepsTaken };
}

test('CursorTrail - Spring damper converges to target position', () => {
  const start = 0;
  const target = 50;
  const { finalX, stepsTaken } = simulateSpringDamper(start, target);

  assert.strictEqual(Math.abs(finalX - target) < 0.1, true);
  assert.strictEqual(stepsTaken <= 60, true);
});
