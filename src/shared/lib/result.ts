/**
 * Result<T, E> for operations that can fail in expected ways.
 * Used in places where throwing would force callers to wrap in try/catch
 * (e.g. slug collisions, validation, parser edge cases).
 */

export type Result<T, E = Error> =
  | { ok: true; value: T }
  | { ok: false; error: E };

export const ok = <T>(value: T): Result<T, never> => ({ ok: true, value });
export const err = <E>(error: E): Result<never, E> => ({ ok: false, error });