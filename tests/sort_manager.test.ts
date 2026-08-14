import { test } from 'node:test';
import assert from 'node:assert';
import { SortManager } from '../src/services/SortManager';
import { TerminalSessionInfo } from '../src/types/terminal';

test('SortManager - Running Priority ordering', () => {
  const manager = new SortManager('running_priority');

  const list: TerminalSessionInfo[] = [
    { id: 't1', title: 'Idle 1', cwd: '/', pid: 1, state: 'IDLE' },
    { id: 't2', title: 'Running 1', cwd: '/', pid: 2, state: 'RUNNING' },
    { id: 't3', title: 'Streaming 1', cwd: '/', pid: 3, state: 'STREAMING' },
    { id: 't4', title: 'Terminated 1', cwd: '/', pid: 4, state: 'TERMINATED' }
  ];

  const sorted = manager.sort(list);
  assert.strictEqual(sorted[0].id === 't2' || sorted[0].id === 't3', true);
  assert.strictEqual(sorted[1].id === 't2' || sorted[1].id === 't3', true);
  assert.strictEqual(sorted[2].id === 't1' || sorted[2].id === 't4', true);
  assert.strictEqual(sorted[3].id === 't1' || sorted[3].id === 't4', true);
});

test('SortManager - MRU activity ordering', async () => {
  const manager = new SortManager('mru');

  const list: TerminalSessionInfo[] = [
    { id: 't1', title: 'Term 1', cwd: '/', pid: 1, state: 'IDLE' },
    { id: 't2', title: 'Term 2', cwd: '/', pid: 2, state: 'IDLE' },
    { id: 't3', title: 'Term 3', cwd: '/', pid: 3, state: 'IDLE' }
  ];

  manager.recordActivity('t1');
  await new Promise((r) => setTimeout(r, 10));
  manager.recordActivity('t3');

  const sorted = manager.sort(list);
  assert.strictEqual(sorted[0].id, 't3');
  assert.strictEqual(sorted[1].id, 't1');
  assert.strictEqual(sorted[2].id, 't2');
});

test('SortManager - Creation timestamp ordering', () => {
  const manager = new SortManager('creation');

  const list: TerminalSessionInfo[] = [
    { id: 't1', title: 'Old', cwd: '/', pid: 1, state: 'IDLE', createdAt: '2026-08-14T10:00:00Z' },
    { id: 't2', title: 'Newest', cwd: '/', pid: 2, state: 'IDLE', createdAt: '2026-08-14T12:00:00Z' },
    { id: 't3', title: 'Middle', cwd: '/', pid: 3, state: 'IDLE', createdAt: '2026-08-14T11:00:00Z' }
  ];

  const sorted = manager.sort(list);
  assert.strictEqual(sorted[0].id, 't2');
  assert.strictEqual(sorted[1].id, 't3');
  assert.strictEqual(sorted[2].id, 't1');
});

test('SortManager - Pinned item always stays at top', () => {
  const manager = new SortManager('creation');
  manager.pinToTop('t1');

  const list: TerminalSessionInfo[] = [
    { id: 't1', title: 'Pinned Old', cwd: '/', pid: 1, state: 'IDLE', createdAt: '2026-08-14T10:00:00Z' },
    { id: 't2', title: 'Newest', cwd: '/', pid: 2, state: 'IDLE', createdAt: '2026-08-14T12:00:00Z' }
  ];

  const sorted = manager.sort(list);
  assert.strictEqual(sorted[0].id, 't1');
  assert.strictEqual(sorted[1].id, 't2');
});
