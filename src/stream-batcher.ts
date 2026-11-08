// Keep the first token immediate; merge later bursts into at most one update
// per 32 ms. There is no timer while idle, and closing preserves the last chunk.
export function createChunkBatcher(
  deliver: (delta: string) => void,
  schedule: (flush: () => void) => (() => void) = flush => {
    const timer = setTimeout(flush, 32);
    return () => clearTimeout(timer);
  },
) {
  let first = true;
  let closed = false;
  let pending = '';
  let cancelTimer: (() => void) | undefined;

  function flush() {
    cancelTimer?.(); cancelTimer = undefined;
    if (!pending) return;
    const delta = pending; pending = '';
    deliver(delta);
  }

  return {
    push(delta: string) {
      if (closed || !delta) return;
      pending += delta;
      if (first) { first = false; flush(); }
      else if (!cancelTimer) cancelTimer = schedule(flush);
    },
    close() {
      closed = true;
      flush();
    },
  };
}
