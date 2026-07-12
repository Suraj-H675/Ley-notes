/**
 * Trailing-edge debounce. The classic use: typing in a CM6 editor triggers a
 * save after the user pauses, not on every keystroke.
 */

export function debounce<TArgs extends unknown[]>(
  fn: (...args: TArgs) => void,
  delayMs: number,
): {
  (...args: TArgs): void;
  cancel: () => void;
  flush: () => void;
} {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pendingArgs: TArgs | null = null;

  const wrapped = (...args: TArgs) => {
    pendingArgs = args;
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      if (pendingArgs) {
        const a = pendingArgs;
        pendingArgs = null;
        fn(...a);
      }
    }, delayMs);
  };

  wrapped.cancel = () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    pendingArgs = null;
  };

  wrapped.flush = () => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
    if (pendingArgs) {
      const a = pendingArgs;
      pendingArgs = null;
      fn(...a);
    }
  };

  return wrapped;
}