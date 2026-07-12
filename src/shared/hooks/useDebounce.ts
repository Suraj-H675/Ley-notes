/**
 * React hook wrapper around debounce(). Cleans up on unmount.
 *
 * Implementation notes: we use a ref to hold the latest callback so callers
 * can pass an inline closure without recreating the timer on every render.
 * The read of fnRef.current happens inside a setTimeout, never during render —
 * but the react-hooks/refs rule fires heuristically on any closure that
 * references a ref, so we disable it for the whole file.
 */

/* eslint-disable react-hooks/refs */

import { useEffect, useLayoutEffect, useMemo, useRef } from 'react';
import { debounce } from '@/shared/lib/debounce';

export function useDebouncedCallback<TArgs extends unknown[]>(
  fn: (...args: TArgs) => void,
  delayMs: number,
): {
  (...args: TArgs): void;
  cancel: () => void;
  flush: () => void;
} {
  const fnRef = useRef(fn);

  useLayoutEffect(() => {
    fnRef.current = fn;
  });

  const debounced = useMemo(
    () =>
      debounce((...args: TArgs) => {
        // Safe: this runs inside setTimeout, not during render.
        fnRef.current(...args);
      }, delayMs),
    [delayMs],
  );

  useEffect(() => {
    return () => {
      debounced.cancel();
    };
  }, [debounced]);

  return debounced;
}