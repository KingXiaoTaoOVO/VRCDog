export class SerialTaskQueue {
  private tail: Promise<unknown> = Promise.resolve();
  private pendingCount = 0;

  constructor(private readonly onPendingChange?: (pending: number) => void) {}

  get pending() {
    return this.pendingCount;
  }

  enqueue<T>(task: () => Promise<T>): Promise<T> {
    this.pendingCount += 1;
    this.onPendingChange?.(this.pendingCount);

    const result = this.tail.then(task, task);
    this.tail = result.then(
      () => undefined,
      () => undefined,
    ).finally(() => {
      this.pendingCount -= 1;
      this.onPendingChange?.(this.pendingCount);
    });
    return result;
  }
}
