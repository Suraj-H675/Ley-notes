import { describe, it, expect, beforeEach } from 'vitest';
import 'fake-indexeddb/auto';
import { renderHook, act, waitFor } from '@testing-library/react';
import { db } from '@/lib/db';
import { useGraphSettings } from './useGraphSettings';

describe('useGraphSettings', () => {
  beforeEach(async () => {
    await db.graphSettings.clear();
  });

  it('returns null initially then seeds defaults', async () => {
    const { result } = renderHook(() => useGraphSettings('global'));
    await waitFor(() => {
      expect(result.current.settings).not.toBeNull();
    });
    expect(result.current.settings?.colorScheme).toBe('untyped');
  });

  it('seeds defaults for both scopes on first call', async () => {
    renderHook(() => useGraphSettings('global'));
    await waitFor(async () => {
      const count = await db.graphSettings.count();
      expect(count).toBe(2);
    });
  });

  it('updates the underlying row when update is called', async () => {
    const { result } = renderHook(() => useGraphSettings('global'));
    await waitFor(() => expect(result.current.settings).not.toBeNull());
    await act(async () => {
      await result.current.update({
        ...(result.current.settings as any),
        colorScheme: 'tag',
      });
    });
    await waitFor(() => {
      expect(result.current.settings?.colorScheme).toBe('tag');
    });
  });

  it('returns separate settings for each scope', async () => {
    const { result: globalHook } = renderHook(() => useGraphSettings('global'));
    const { result: localHook } = renderHook(() => useGraphSettings('local'));
    await waitFor(() => {
      expect(globalHook.current.settings).not.toBeNull();
      expect(localHook.current.settings).not.toBeNull();
    });
    expect(globalHook.current.settings?.scope).toBe('global');
    expect(localHook.current.settings?.scope).toBe('local');
  });
});
