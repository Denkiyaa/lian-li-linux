type BrightnessStatus = { request_id?: string | null; brightness: number; pending: boolean; error: string | null };

export function brightnessConfirmed(status: BrightnessStatus | undefined, requestId?: string): boolean {
  return !!requestId && status?.request_id === requestId && !status.pending && status.error === null;
}

export function brightnessError(
  value: number,
  status: BrightnessStatus | undefined,
  requestError?: string,
  requestId?: string,
): string | undefined {
  if (requestId && status?.request_id === requestId && status.brightness === value && !status.pending) {
    return status.error ?? undefined;
  }
  return requestError ?? (status?.brightness === value ? status.error ?? undefined : undefined);
}

export function createBrightnessControl(
  send: (deviceId: string, value: number) => Promise<unknown>,
  onError: (deviceId: string, error: unknown) => void,
) {
  const pending = new Map<string, number>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let running = false;
  let disposed = false;

  async function flush() {
    timer = undefined;
    const next = pending.entries().next().value;
    if (disposed || running || !next) return;
    const [deviceId, value] = next;
    pending.delete(deviceId);
    running = true;
    try {
      await send(deviceId, value);
    } catch (error) {
      if (!disposed && !pending.has(deviceId)) onError(deviceId, error);
    } finally {
      running = false;
      if (!disposed && pending.size) timer = setTimeout(flush, 200);
    }
  }

  return {
    set(deviceId: string, value: number) {
      if (disposed) return;
      pending.set(deviceId, value);
      clearTimeout(timer);
      if (!running) timer = setTimeout(flush, 200);
    },
    dispose() {
      disposed = true;
      clearTimeout(timer);
      pending.clear();
    },
  };
}
