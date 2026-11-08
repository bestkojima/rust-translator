import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { createChunkBatcher } from '../src/stream-batcher.ts';

function setup() {
  const output: string[] = [];
  const timers = new Set<() => void>();
  const batch = createChunkBatcher(delta => output.push(delta), flush => {
    timers.add(flush);
    return () => timers.delete(flush);
  });
  return { batch, output, timers, tick: () => { for (const timer of [...timers]) timer(); } };
}

test('first chunk is immediate, bursts use one timer and preserve exact text', () => {
  const { batch, output, timers, tick } = setup();
  batch.push('你');
  assert.deepEqual(output, ['你']);
  assert.equal(timers.size, 0);
  for (const delta of ['好', '，', '世', '界', '🌏']) batch.push(delta);
  assert.equal(timers.size, 1);
  tick();
  assert.deepEqual(output, ['你', '好，世界🌏']);
  assert.equal(timers.size, 0);
});

test('completion or cancellation flushes the tail once and ignores late events', () => {
  const { batch, output, timers, tick } = setup();
  batch.push('first'); batch.push(' tail');
  batch.close(); batch.close(); batch.push('stale'); tick();
  assert.equal(output.join(''), 'first tail');
  assert.equal(timers.size, 0);
});

test('empty events create no work; a 1000-token burst uses two deliveries', () => {
  const { batch, output, timers } = setup();
  batch.push(''); assert.equal(timers.size, 0);
  for (let i = 0; i < 1000; i++) batch.push('译');
  batch.close();
  assert.equal(output.length, 2);
  assert.equal(output.join(''), '译'.repeat(1000));
  assert.equal(timers.size, 0);
});

test('synthetic 100-chunk/s stream batches updates without delaying its first chunk', () => {
  let now = 0;
  const timers = new Map<() => void, number>();
  const output: string[] = [];
  const batch = createChunkBatcher(delta => output.push(delta), flush => {
    timers.set(flush, now + 32);
    return () => timers.delete(flush);
  });
  for (let i = 0; i < 1000; i++) {
    now = i * 10;
    for (const [timer, due] of [...timers]) if (due <= now) timer();
    batch.push('字');
    if (i === 0) assert.deepEqual(output, ['字']);
  }
  batch.close();
  assert.equal(output.length, 251);
  assert.equal(output.join(''), '字'.repeat(1000));
  assert.equal(timers.size, 0);
});
