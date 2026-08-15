import { describe, expect, it, vi } from 'vitest';
import { SerialTaskQueue } from './serialTaskQueue';

describe('SerialTaskQueue', () => {
  it('preserves submission order even when later work is faster', async () => {
    const order: number[] = [];
    const queue = new SerialTaskQueue();
    const first = queue.enqueue(async () => {
      await new Promise(resolve => setTimeout(resolve, 10));
      order.push(1);
      return 'first';
    });
    const second = queue.enqueue(async () => {
      order.push(2);
      return 'second';
    });

    await expect(Promise.all([first, second])).resolves.toEqual(['first', 'second']);
    expect(order).toEqual([1, 2]);
  });

  it('continues after a failed task and reports pending work', async () => {
    const onPending = vi.fn();
    const queue = new SerialTaskQueue(onPending);
    const failed = queue.enqueue(async () => { throw new Error('failed'); });
    const recovered = queue.enqueue(async () => 'ok');

    await expect(failed).rejects.toThrow('failed');
    await expect(recovered).resolves.toBe('ok');
    expect(queue.pending).toBe(0);
    expect(onPending).toHaveBeenLastCalledWith(0);
  });
});
